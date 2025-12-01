use std::{net::SocketAddr, path::PathBuf};

use clap::{Args, ValueEnum};

use crate::parse_addr_with_default_port;

#[derive(Debug, Args)]
pub struct TransportConfig {
    #[arg(long = "transport", short = 't', default_value = "quic")]
    pub transport_type: TransportType,
    #[clap(flatten)]
    pub transport_params: TransportParams,
    #[arg(long, default_value = "700")]
    pub keepalive_timeout: u64,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum TransportType {
    Quic,
    Tcp,
    Kcp,
    Websocket,
}

#[derive(Debug, Args)]
pub struct TransportParams {
    #[arg(long, short = 's', value_parser = parse_addr_with_default_port::<7777>)]
    pub server_addr: SocketAddr,
    #[arg(long)]
    pub cert_path: Option<PathBuf>,
    #[arg(long)]
    pub hostname: Option<String>,
    #[arg(long)]
    pub no_delay: Option<bool>,
    #[arg(long)]
    pub no_tls: bool,
}
