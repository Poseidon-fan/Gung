mod run;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

pub use run::*;

#[derive(Debug, Parser)]
#[command(name = "gungs", version, about = "Gung server")]
pub struct CliConfig {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Run { config_file_path: PathBuf },
}
