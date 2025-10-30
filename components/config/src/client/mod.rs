mod proxy;
mod transport;

use std::net::SocketAddr;

use clap::Parser;

pub use proxy::*;
use serde_json::Value as JsonValue;
pub use transport::*;

use crate::parse_addr_with_default_host;

#[derive(Debug, Parser)]
#[command(name = "gungc", about = "Gung client")]
pub struct CliConfig {
    #[arg(
        value_name = "HOST:PORT | PORT",
        value_parser = parse_addr_with_default_host::<127, 0, 0, 1>
    )]
    pub local_addr: SocketAddr,

    #[clap(flatten)]
    pub proxy: ProxyConfig,

    #[clap(flatten)]
    pub transport: TransportConfig,

    #[arg(long, short = 'd')]
    pub data: Option<JsonValue>,
}
