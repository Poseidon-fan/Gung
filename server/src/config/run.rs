use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RunConfig {
    pub transport: TransportConfig,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportConfig {
    Quic(QuicTransportConfig),
    Tcp(TcpTransportConfig),
}

#[derive(Debug, Deserialize)]
pub struct QuicTransportConfig {}

#[derive(Debug, Deserialize)]
pub struct TcpTransportConfig {}
