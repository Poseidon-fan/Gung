use std::fs;

use anyhow::Result;
use clap::Parser;
use server::config::{CliConfig, Commands, RunConfig};

fn main() -> Result<()> {
    let cli_config = CliConfig::parse();
    match cli_config.command {
        Commands::Run { config_file_path } => {
            let run_config: RunConfig = toml::from_str(&fs::read_to_string(config_file_path)?)?;
            server::run_server(run_config)
        }
    }
}
