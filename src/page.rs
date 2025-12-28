use crate::git::get_git_files;
use crate::media_type::media_type_from_path;
use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::RwLock;
use tokio::task::spawn_blocking;

pub struct Page {
    // Settings
    pub repo: String,
    pub git_ref: Option<String>,
    pub subfolder: Option<String>,
    pub max_bytes: Option<u64>,
    pub prefix: String,
    pub auto_index: bool,
    pub auto_list: bool,
    pub update_secret: Option<String>,

    // State
    pub last_hash: Option<String>,
    pub files: Vec<PageFile>,
}

pub struct PageFile {
    pub path: String,
    pub media_type: String,
    pub hash: String,
    pub data: Vec<u8>,
}

impl Page {
    pub fn from_env(number: Option<usize>) -> Option<Self> {
        let page = if let Some(number) = number {
            format!("PAGE{number}")
        } else {
            String::from("PAGE")
        };
        let repo = std::env::var(format!("{page}_GIT_REPO")).ok()?;
        let git_ref = std::env::var(format!("{page}_GIT_REF")).ok();
        let subfolder = std::env::var(format!("{page}_GIT_SUBFOLDER")).ok();
        let max_bytes = std::env::var(format!("{page}_MAX_BYTES"))
            .ok()
            .and_then(|s| s.parse::<u64>().ok());
        let prefix = std::env::var(format!("{page}_PREFIX")).unwrap_or(String::from("/"));
        let auto_index = std::env::var(format!("{page}_AUTO_INDEX"))
            .ok()
            .map(|s| s.to_lowercase())
            .map(|s| s == "true" || s == "on" || s == "enabled")
            .unwrap_or(true);
        let auto_list = std::env::var(format!("{page}_AUTO_LIST"))
            .ok()
            .map(|s| s.to_lowercase())
            .map(|s| s == "true" || s == "on" || s == "enabled")
            .unwrap_or(false);
        let update_secret = std::env::var(format!("{page}_UPDATE_SECRET")).ok();
        Some(Self {
            repo,
            git_ref,
            subfolder,
            max_bytes,
            prefix,
            auto_index,
            auto_list,
            update_secret,
            last_hash: None,
            files: Vec::new(),
        })
    }

    pub fn from_cli(number: Option<usize>) -> Option<Self> {
        let page = if let Some(number) = number {
            format!("page{number}")
        } else {
            String::from("page")
        };
        let repo = get_cli_arg(format!("{page}-git-repo"))?;
        let git_ref = get_cli_arg(format!("{page}-git-ref"));
        let subfolder = get_cli_arg(format!("{page}-git-subfolder"));
        let max_bytes =
            get_cli_arg(format!("{page}-max-bytes")).and_then(|s| s.parse::<u64>().ok());
        let prefix = get_cli_arg(format!("{page}-prefix")).unwrap_or(String::from("/"));
        let auto_index = get_cli_arg(format!("{page}-auto-index"))
            .map(|s| s.to_lowercase())
            .map(|s| s == "true" || s == "on" || s == "enabled")
            .unwrap_or(true);
        let auto_list = get_cli_arg(format!("{page}-auto-list"))
            .map(|s| s.to_lowercase())
            .map(|s| s == "true" || s == "on" || s == "enabled")
            .unwrap_or(false);
        let update_secret = get_cli_arg(format!("{page}-update-secret"));
        Some(Self {
            repo,
            git_ref,
            subfolder,
            max_bytes,
            prefix,
            auto_index,
            auto_list,
            update_secret,
            last_hash: None,
            files: Vec::new(),
        })
    }

    pub fn find_file(&self, path: &str) -> Option<&PageFile> {
        // Check for any index files
        if self.auto_index && path.ends_with("/") {
            let names = [
                "index.html",
                "index.htm",
                "default.html",
                "default.htm",
                "home.html",
                "home.htm",
            ];
            for name in names {
                let index_path = format!("{path}{name}");
                if let Some(file) = self.files.iter().find(|&f| index_path == f.path) {
                    return Some(file);
                }
            }
        }

        // Normal direct search
        self.files.iter().find(|&f| path == f.path)
    }

    pub fn list_folder(&self, path: &str) -> Option<String> {
        // Helper struct for listed entries
        struct ListEntry {
            size: Option<usize>,
            hash: Option<String>,
        }

        // Collect files and folders of current path
        let mut entries = HashMap::new();
        for file in &self.files {
            if let Some(stripped) = file.path.strip_prefix(path) {
                if let Some((child, _)) = stripped.split_once("/") {
                    // Folder
                    let link = format!("{child}/");
                    entries.insert(
                        link,
                        ListEntry {
                            size: None,
                            hash: None,
                        },
                    );
                } else {
                    // Normal file
                    let link = stripped.to_string();
                    entries.insert(
                        link,
                        ListEntry {
                            size: Some(file.data.len()),
                            hash: Some(file.hash.clone()),
                        },
                    );
                }
            }
        }

        // Early out in case there are no files or folders
        if entries.is_empty() {
            return None;
        }

        // Add parent folder if we are not already in the root
        if path != "/" {
            entries.insert(
                String::from("../"),
                ListEntry {
                    size: None,
                    hash: None,
                },
            );
        }

        // Sort all links alphabetically
        let mut links: Vec<&String> = entries.keys().collect();
        links.sort();

        // Generate HTML
        let mut html = format!(
            r#"
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="utf-8">
                <title>Contents of {path}</title>
                <link rel="icon" href="data:image/png;base64,iVBORw0KGgo=">
                <style>
                    body {{ font-family: monospace; }}
                    th, td {{ text-align: left; padding-right: 20px; }}
                </style>
            </head>
            <body>
                <h1>Contents of {path}</h1>
                <table>
                    <tr>
                        <th>&nbsp;</th>
                        <th>Item</th>
                        <th>Size</th>
                        <th>Hash</th>
                    </tr>
        "#
        );
        for link in links {
            let symbol = if link.ends_with("/") {
                "&#128193;"
            } else {
                "&#128196;"
            };
            let Some(details) = entries.get(link) else {
                continue;
            };
            let size = details
                .size
                .map(|s| s.to_string())
                .unwrap_or(String::from("&nbsp;"));
            let hash = details
                .hash
                .as_deref()
                .map(|s| s.to_string())
                .unwrap_or(String::from("&nbsp;"));
            html.push_str(&format!(
                r#"
                <tr>
                    <td>{symbol}</td>
                    <td><a href="{path}{link}">{link}</a></td>
                    <td>{size}</td>
                    <td>{hash}</td>
                </tr>
            "#
            ));
        }
        html.push_str("</table></body></html>");
        Some(html)
    }
}

pub async fn update_page(page_lock: &RwLock<Page>, temp_folder: &Path) -> Result<()> {
    let page = page_lock.read().await;
    let repo = page.repo.clone();
    let last_hash = page.last_hash.clone();
    let reference = page.git_ref.clone();
    let max_bytes = page.max_bytes;
    let subfolder = page.subfolder.clone();
    let prefix = page.prefix.clone();
    drop(page);

    // Prepare folder path to be used for git bare clone
    let stripped_prefix = prefix
        .strip_prefix("/")
        .context("Failed to remove leading slash from prefix")?;
    let folder = if stripped_prefix.is_empty() {
        "root/"
    } else {
        stripped_prefix
    };
    let temp_folder = temp_folder.to_path_buf().join(folder);

    let handle =
        spawn_blocking(move || get_git_files(&repo, reference.as_deref(), &temp_folder, max_bytes));
    let result = handle
        .await
        .context("Failed to join blocking update task")?;
    let checkout = result.context("Failed to get git files")?;

    if let Some(hash) = last_hash
        && checkout.hash == hash
    {
        // Early out, git ref has not changed!
        return Ok(());
    }

    let mut new_files = Vec::new();
    for file in checkout.files {
        if let Some(folder) = &subfolder {
            // Filter out only files from subfolder with reduced paths
            if let Some(path) = file.path.strip_prefix(folder) {
                new_files.push(PageFile {
                    path: format!("{}{}", prefix, path),
                    media_type: media_type_from_path(path).to_string(),
                    hash: file.hash,
                    data: file.data,
                });
            }
        } else {
            // All files are added unfiltered
            new_files.push(PageFile {
                path: format!("{}{}", prefix, file.path),
                media_type: media_type_from_path(&file.path).to_string(),
                hash: file.hash,
                data: file.data,
            });
        }
    }

    if new_files.is_empty() {
        bail!("No files found")
    }

    let mut page = page_lock.write().await;
    page.files = new_files;
    page.last_hash = Some(checkout.hash);
    Ok(())
}

fn get_cli_arg(name: String) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    for arg in args {
        let prefix = format!("--{name}=");
        if let Some(value) = arg.trim().strip_prefix(&prefix) {
            return Some(value.to_string());
        }
    }

    None
}
