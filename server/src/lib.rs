pub mod config;
mod tls;

use anyhow::{Ok, Result};

use crate::config::RunConfig;

#[tokio::main]
pub async fn run_server(run_config: RunConfig) -> Result<()> {
    println!("run_config: {run_config:?}");
    Ok(())
}
