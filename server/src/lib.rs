#![allow(dead_code)]
mod cert;
mod port;
mod proxy;
mod service;

use anyhow::{Ok, Result};

use config::server::RunConfig;

#[tokio::main]
pub async fn run_server(run_config: RunConfig) -> Result<()> {
    println!("run_config: {run_config:?}");
    Ok(())
}
