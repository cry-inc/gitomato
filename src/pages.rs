use crate::page::{Page, update_page};
use anyhow::{Result, bail, ensure};
use std::path::Path;
use tokio::sync::RwLock;
use tokio::time::Instant;
use tracing::{info, warn};

pub struct Pages {
    pages: Vec<RwLock<Page>>,
}

impl Pages {
    pub async fn from_cli_and_env() -> Result<Self> {
        // Check for single page without numbers
        let mut pages = Self { pages: Vec::new() };
        if let Some(page) = Page::from_cli(None) {
            pages.add_page(page).await?;
        } else if let Some(page) = Page::from_env(None) {
            pages.add_page(page).await?;
        }
        if !pages.pages.is_empty() {
            return Ok(pages);
        }

        // Number pages from 0..n
        let mut n = 0;
        loop {
            if let Some(page) = Page::from_cli(Some(n)) {
                pages.add_page(page).await?;
            } else if let Some(page) = Page::from_env(Some(n)) {
                pages.add_page(page).await?;
            } else {
                break;
            }
            n += 1;
        }

        Ok(pages)
    }

    async fn add_page(&mut self, new_page: Page) -> Result<()> {
        ensure!(
            new_page.prefix.starts_with("/"),
            "Prefix must start with slash"
        );
        ensure!(new_page.prefix.ends_with("/"), "Prefix must end with slash");
        for page_lock in &self.pages {
            let page = page_lock.read().await;
            if page.prefix == new_page.prefix {
                bail!("Prefix {} is already used by another page", page.prefix);
            }
            if page.prefix.starts_with(&new_page.prefix)
                || new_page.prefix.starts_with(&page.prefix)
            {
                bail!(
                    "Existing prefix {} conflicts with new prefix {}",
                    page.prefix,
                    new_page.prefix
                );
            }
        }
        self.pages.push(RwLock::new(new_page));
        Ok(())
    }

    pub async fn update(&self, temp_folder: &Path) {
        for page_lock in &self.pages {
            let start = Instant::now();
            let result = update_page(page_lock, temp_folder).await;
            let duration = start.elapsed();
            let page = page_lock.read().await;
            if let Err(error) = result {
                warn!(
                    "Failed to update page {} from repo {} for ref {:?} after {:?}: {:#}",
                    page.prefix, page.repo, page.git_ref, duration, error
                );
            } else {
                info!(
                    "Updated page {} from repo {} for ref {:?} after {:?}",
                    page.prefix, page.repo, page.git_ref, duration
                );
            }
        }
    }

    pub async fn find_page(&self, path: &str) -> Option<&RwLock<Page>> {
        for page_lock in &self.pages {
            let page = page_lock.read().await;
            if path.starts_with(&page.prefix) {
                return Some(page_lock);
            }
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }

    pub async fn log(&self) {
        let b2o = |value: bool| -> &'static str {
            match value {
                true => "on",
                false => "off",
            }
        };
        info!("Found {} configured pages", self.pages.len());
        for (i, page_lock) in self.pages.iter().enumerate() {
            let page = page_lock.read().await;
            info!(
                "Page {i} running on path {} for repo {} and ref {:?}",
                page.prefix, page.repo, page.git_ref
            );
            info!(
                "Page {i} has auto index {} and auto list {}",
                b2o(page.auto_index),
                b2o(page.auto_list)
            );
            if let Some(max) = page.max_bytes {
                info!("Page {i} has a max limit of {max} bytes configured");
            }
            if let Some(folder) = &page.subfolder {
                info!("Page {i} is limited to subfolder {folder}");
            }
            if let Some(secret) = &page.update_secret {
                info!(
                    "Page {i} has a GET update hook enabled at {}update/{secret}",
                    page.prefix
                );
            }
        }
    }
}
