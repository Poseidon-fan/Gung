use std::{net::SocketAddr, path::PathBuf};

use serde::Deserialize;

use crate::{default_bool, default_u64};

#[derive(Debug, Deserialize)]
pub struct TransportConfig {
    #[serde(default = "default_addr")]
    pub addr: SocketAddr,

    #[serde(flatten)]
    pub protocol: ProtocolConfig,

    #[serde(flatten, default)]
    pub keepalive: KeepaliveConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct KeepaliveConfig {
    #[serde(default = "default_u64::<600>")]
    pub keepalive_interval: u64,
    #[serde(default = "default_u64::<700>")]
    pub keepalive_timeout: u64,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ProtocolConfig {
    #[serde(rename = "tcp")]
    Tcp(TcpTransportConfig),
    #[serde(rename = "quic")]
    Quic(QuicTransportConfig),
    #[serde(rename = "kcp")]
    Kcp(KcpTransportConfig),
    #[serde(rename = "websocket")]
    Websocket(WebsocketTransportConfig),
}

#[derive(Debug, Deserialize)]
pub struct TcpTransportConfig {
    #[serde(default = "default_bool::<true>")]
    pub no_delay: bool,
    #[serde(flatten)]
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Deserialize, Default)]
pub struct QuicTransportConfig {
    #[serde(flatten)]
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Deserialize)]
pub struct KcpTransportConfig {
    #[serde(default = "default_bool::<true>")]
    pub no_delay: bool,
    #[serde(flatten)]
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Deserialize)]
pub struct WebsocketTransportConfig {
    #[serde(flatten)]
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Deserialize)]
pub struct TlsConfig {
    pub cert: PathBuf,
    pub key: PathBuf,
}

fn default_addr() -> SocketAddr {
    "0.0.0.0:7777".parse().unwrap()
}
