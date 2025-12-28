use clap::Parser;
use std::{net::IpAddr, path::PathBuf};
use tracing::{Level, info};

#[derive(Parser, Clone)]
#[command(version, about, long_about = None, ignore_errors(true))]
pub struct Configuration {
    /// HTTP server port
    #[arg(long, env, default_value_t = 8080)]
    pub http_port: u16,

    /// HTTP server binding address
    #[arg(long, env, default_value = "0.0.0.0")]
    pub http_binding: IpAddr,

    /// Temporary folder used to store bare git checkouts druing updates
    #[arg(long, env, default_value = "./temp")]
    pub temp_folder: PathBuf,

    /// Logging level
    #[arg(long, env, default_value = "INFO")]
    pub log_level: Level,

    /// Background update interval for check the git repos in seconds
    #[arg(long, env, default_value_t = 300)]
    pub interval: u64,
}

impl Configuration {
    pub fn log(&self) {
        info!("HTTP Port: {}", self.http_port);
        info!("HTTP Binding: {}", self.http_binding);
        info!("Temp Folder: {}", self.temp_folder.display());
        info!("Log Level: {}", self.log_level);
        info!("Update Interval: {} sec", self.interval);
    }
}
