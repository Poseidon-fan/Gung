use std::{net::SocketAddr, path::PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TransportConfig {
    #[serde(default = "default_addr")]
    pub addr: SocketAddr,

    #[serde(flatten)]
    pub protocol: ProtocolConfig,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ProtocolConfig {
    #[serde(rename = "tcp")]
    Tcp(TcpTransportConfig),
    #[serde(rename = "quic")]
    Quic(QuicTransportConfig),
}

#[derive(Debug, Deserialize)]
pub struct TcpTransportConfig {}

#[derive(Debug, Deserialize, Default)]
pub struct QuicTransportConfig {
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
}

fn default_addr() -> SocketAddr {
    "0.0.0.1:7777".parse().unwrap()
}
