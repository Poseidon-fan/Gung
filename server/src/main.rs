use std::fs;

use anyhow::Result;
use clap::Parser;
use config::server::{CliConfig, Command, RunConfig};
use tracing::debug;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    const DEFAULT_LEVEL: &str = "info";
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::from(DEFAULT_LEVEL)),
        )
        .with_ansi(atty::is(atty::Stream::Stdout))
        .init();

    let cli_config = CliConfig::parse();
    debug!(?cli_config);

    match cli_config.command {
        Command::Run { config_file_path } => {
            let run_config: RunConfig = toml::from_str(&fs::read_to_string(config_file_path)?)?;
            debug!(?run_config);
            server::run_server(run_config)
        }
    }
}
