use std::net::SocketAddr;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TransportConfig {
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

#[derive(Debug, Deserialize)]
pub struct QuicTransportConfig {}
