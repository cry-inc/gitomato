use crate::config::Configuration;
use crate::page::{Page, PageFile, update_page};
use crate::pages::Pages;
use anyhow::{Context, Result};
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::header::IF_NONE_MATCH;
use hyper::server::conn::http1::Builder;
use hyper::service::service_fn;
use hyper::{Method, Request, Response};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal::ctrl_c;
use tokio::sync::RwLock;
use tokio::time::Instant;
use tokio::{select, spawn};
use tracing::{info, warn};

pub async fn start_server(config: Arc<Configuration>, pages: Arc<Pages>) -> Result<()> {
    let addr = SocketAddr::from((config.http_binding, config.http_port));
    let listener = TcpListener::bind(addr)
        .await
        .context("Failed to bind TCP address")?;
    loop {
        let stream = select! {
            result = listener.accept() => {
                if let Ok((stream, _)) = result {
                    stream
                } else {
                    warn!("Failed to accept request");
                    continue;
                }
            },
            _ = shutdown_signal() => { return Ok(()); },
        };
        let io = TokioIo::new(stream);
        let pages = pages.clone();
        let config = config.clone();
        spawn(async move {
            if let Err(err) = Builder::new()
                .serve_connection(
                    io,
                    service_fn(|req: Request<Incoming>| {
                        let pages = pages.clone();
                        let config = config.clone();
                        async move { root_handler(req, config, pages).await }
                    }),
                )
                .await
            {
                warn!("Serving error: {err:?}");
            }
        });
    }
}

async fn shutdown_signal() {
    let ctrlc = async {
        ctrl_c().await.expect("Failed to install CTRL+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrlc => {},
        _ = terminate => {},
    }
}

async fn root_handler(
    req: Request<Incoming>,
    config: Arc<Configuration>,
    pages: Arc<Pages>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let uri = req.uri();
    let path = uri.path();
    if req.method() == Method::GET
        && let Some(page_lock) = pages.find_page(path).await
    {
        let page = page_lock.read().await;
        if let Some(secret) = &page.update_secret {
            let update_path = format!("{}update/{}", page.prefix, secret);
            if path == update_path {
                let page_prefix = page.prefix.clone();
                drop(page);
                return update_handler(config, &page_prefix, page_lock).await;
            }
        }
        if let Some(file) = page.find_file(path) {
            return file_handler(req, file).await;
        }
        if page.auto_list
            && path.ends_with("/")
            && let Some(html) = page.list_folder(path)
        {
            let bytes = Bytes::from(html);
            let body = Full::new(bytes);
            let response = Response::builder()
                .status(200)
                .header("Content-Type", "text/html")
                .body(body)
                .expect("Failed to build HTTP response");
            return Ok(response);
        }
    }
    let body = Bytes::from_static(b"Not found");
    let response = Response::builder()
        .status(404)
        .body(Full::new(body))
        .expect("Failed to build HTTP response");
    Ok(response)
}

async fn file_handler(
    req: Request<Incoming>,
    file: &PageFile,
) -> Result<Response<Full<Bytes>>, Infallible> {
    // Handle If-None-Match requests for current ETag
    if let Some(value) = req.headers().get(IF_NONE_MATCH)
        && let Ok(str_value) = value.to_str()
        && str_value == file.hash
    {
        let bytes = Bytes::new();
        let body = Full::new(bytes);
        let response = Response::builder()
            .status(304)
            .header("ETag", &file.hash)
            .body(body)
            .expect("Failed to build HTTP response");
        return Ok(response);
    }

    // Return full file
    let bytes = Bytes::from(file.data.clone());
    let body = Full::new(bytes);
    let response = Response::builder()
        .status(200)
        .header("Content-Type", &file.media_type)
        .header("ETag", &file.hash)
        .body(body)
        .expect("Failed to build HTTP response");
    Ok(response)
}

async fn update_handler(
    config: Arc<Configuration>,
    page_prefix: &str,
    page_lock: &RwLock<Page>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let start = Instant::now();
    let result = update_page(page_lock, &config.temp_folder).await;
    let duration = start.elapsed();
    if let Err(err) = result {
        warn!("Update via HTTP handler for {page_prefix} failed after {duration:?}: {err:#}");
        let bytes = Bytes::from_static(b"Update failed");
        let body = Full::new(bytes);
        let response = Response::builder()
            .status(500)
            .body(body)
            .expect("Failed to build HTTP response");
        Ok(response)
    } else {
        info!("Update via HTTP handler for {page_prefix} finished successful after {duration:?}");
        let bytes = Bytes::from_static(b"Update completed");
        let body = Full::new(bytes);
        let response = Response::builder()
            .status(200)
            .body(body)
            .expect("Failed to build HTTP response");
        Ok(response)
    }
}
