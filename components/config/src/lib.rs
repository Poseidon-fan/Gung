use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};

use serde_json::Value as JsonValue;

pub mod client;
pub mod server;

pub(crate) fn parse_addr(s: &str) -> Result<SocketAddr, String> {
    s.to_socket_addrs()
        .map_err(|e| e.to_string())?
        .next()
        .ok_or("Invalid address format (expected host:port)".to_string())
}

pub(crate) fn default_u64<const DEFAULT: u64>() -> u64 {
    DEFAULT
}

pub(crate) fn parse_addr_with_default_host<const A: u8, const B: u8, const C: u8, const D: u8>(
    s: &str,
) -> Result<SocketAddr, String> {
    if s.contains(':') {
        parse_addr(s)
    } else {
        let port = s
            .parse::<u16>()
            .map_err(|e| format!("Invalid port number: {}", e))?;
        Ok(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(A, B, C, D)), port))
    }
}

pub(crate) fn parse_addr_with_default_port<const P: u16>(s: &str) -> Result<SocketAddr, String> {
    if s.contains(':') {
        parse_addr(s)
    } else {
        let addr_str = format!("{s}:{P}");
        parse_addr(&addr_str)
    }
}

fn parse_json(s: &str) -> Result<JsonValue, String> {
    serde_json::from_str(s).map_err(|e| e.to_string())
}
