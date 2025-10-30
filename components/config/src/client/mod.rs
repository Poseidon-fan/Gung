mod proxy;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use clap::Parser;

pub use proxy::*;

#[derive(Debug, Parser)]
#[command(name = "gungc", about = "Gung client")]
pub struct CliConfig {
    #[arg(
        value_name = "HOST:PORT | PORT",
        value_parser = parse_addr_with_default_host
    )]
    pub local_addr: SocketAddr,

    #[clap(flatten)]
    pub proxy: ProxyConfig,
}

fn parse_addr_with_default_host(s: &str) -> Result<SocketAddr, String> {
    const DEFAULT_HOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    if s.contains(':') {
        s.parse::<SocketAddr>()
            .map_err(|e| format!("Invalid address format (expected host:port or port): {}", e))
    } else {
        let port = s
            .parse::<u16>()
            .map_err(|e| format!("Invalid port number: {}", e))?;
        Ok(SocketAddr::new(DEFAULT_HOST, port))
    }
}
