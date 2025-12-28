mod config;
mod git;
mod http;
mod media_type;
mod page;
mod pages;
mod updates;

use crate::config::Configuration;
use crate::http::start_server;
use crate::pages::Pages;
use crate::updates::background_updates;
use anyhow::{Context, Result, bail};
use clap::Parser;
use std::sync::Arc;
use tokio::spawn;
use tokio::sync::mpsc::channel;
use tracing::info;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Load config
    let config = Arc::new(Configuration::parse());

    // Set up logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(config.log_level)
        .with_ansi(false)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global tracing subscriber");
    info!("Starting gitomato version {}...", env!("CARGO_PKG_VERSION"));

    // Log configuration values
    config.log();

    // Set up the different pages
    let pages = Pages::from_cli_and_env()
        .await
        .context("Failed to set up pages")?;
    if pages.is_empty() {
        show_page_setup_help();
        bail!("Need at least one configured page!");
    }
    pages.log().await;

    // Spawn background update task
    let pages = Arc::new(pages);
    let pages_clone = pages.clone();
    let config_clone = config.clone();
    let (stop_sender, stop_receiver) = channel(1);
    let bg_handle =
        spawn(async move { background_updates(pages_clone, config_clone, stop_receiver).await });

    // Start HTTP server
    info!(
        "Starting HTTP server bound to {}:{}...",
        config.http_binding, config.http_port
    );
    start_server(config, pages)
        .await
        .context("Failed to start HTTP server")?;

    // Shutdown rest of app after HTTP server stopped
    info!("HTTP server stopped");
    info!("Stopping background task...");
    stop_sender
        .send(())
        .await
        .expect("Failed to send background task shutdown signal");
    bg_handle.await.expect("Failed to join background task");
    info!("Background task stopped, application shutdown completed!");

    Ok(())
}

fn show_page_setup_help() {
    println!(
        r#"
    You need to configure one or more pages to run this application.

    A page can be configured via command line arguments or environment variables.

    Each page has the following configuration values:
    * PAGE_GIT_REPO or --page-git-repo (required)
      HTTP(S) URL to the git repo. Can include username and password.
      Example: "https://user:pass@git.server.com"
    * PAGE_GIT_REF or --page-git-ref (optional)
      Git branch to be checked out. If omitted, the default branch will be used.
    * PAGE_GIT_SUBFOLDER or --page-git-subfolder (optional)
      Subfolder to check out. Will use the whole repository if not set.
      Example value: "my/sub/folder/"
    * PAGE_PREFIX --page-prefix (optional)
      Where to mount the page on the HTTP server.
      Default is "/". Other example value: "/mypage/".
      Needs to start and end with a slash.
      Keep in mind that there can be only one page at a certain prefix.
    * PAGE_AUTO_INDEX or --page-auto-index (optional)
      When enabled, this will automatically serve any existing index.htm(l),
      default.htm(l) or home.htm(l) file when a folder path is requested.
      In this order. This is enabled by default.
    * PAGE_AUTO_LIST or --page-auto-list (optional)
      When enabled, this will generate a folder listing index with all contained
      files and subfolders for directories without an index page.
      Disabled by default.
    * PAGE_UPDATE_SECRET or --page-update-secret (optional)
      When set, this activates a HTTP GET webhook endpoint for automatic updates.
      When called, this will trigger a git update for this page.
      Example value: "my-SUPER-secr3t"
      Resulting endpoint: GET http://server.com/page-prefix/update/my-SUPER-secr3t
    * PAGE_MAX_BYTES or --page-max-bytes (optional)
      Since all data is kept in memory you can configure a max size in bytes.
      If the checkout of this page is over this limit, the page update will fail.
      By default, there is no limit set.

    Simplest possible setup for a single page using command line arguments:
    gitomato --page-git-repo=https://github.com/user/repo.git

    When you want multiple pages, you need to configure numbered pages starting from 0:
    gitomato --page0-git-repo=https://github.com/user/repo0.git \
             --page1-git-repo=https://github.com/user/repo1.git \
             ...

    You can mix pages from command line arguments and evironment variables,
    but all parameters for a specific page need to come from the same source!

    If you have command line arguments and environment variables for the same page,
    the command line arguments will win and the environment variables will be ignored.

    Run gitomato --help for a reference of all global configuration parameters.
    "#
    );
}
