use clap::Parser;
use config::client::CliConfig;

fn main() {
    let cli_config = CliConfig::parse();
    println!("cli_config: {cli_config:?}");
}
