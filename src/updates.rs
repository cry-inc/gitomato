use crate::config::Configuration;
use crate::pages::Pages;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::time::sleep;
use tracing::info;

pub async fn background_updates(
    pages: Arc<Pages>,
    config: Arc<Configuration>,
    mut stopper: Receiver<()>,
) {
    loop {
        info!("Starting background updates...");
        pages.update(&config.temp_folder).await;
        info!("Finished background updates");

        tokio::select! {
            _ = sleep(Duration::from_secs(config.interval)) => {},
            _ = stopper.recv() => { break; },
        }
    }
}
