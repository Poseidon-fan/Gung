use clap::Parser;
use config::client::CliConfig;

use anyhow::Result;

fn main() -> Result<()> {
    let cli_config = CliConfig::parse();
    client::run_client(cli_config)
}
