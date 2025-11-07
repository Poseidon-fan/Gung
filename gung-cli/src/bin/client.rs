use clap::Parser;
use config::client::CliConfig;

use anyhow::Result;
use tracing::debug;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    let cli_config = CliConfig::parse();

    const DEFAULT_LEVEL: &str = "info";
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::from(DEFAULT_LEVEL)),
        )
        .with_ansi(atty::is(atty::Stream::Stdout))
        .init();
    debug!(?cli_config);

    client::run_client(cli_config)
}
