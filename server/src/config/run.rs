use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RunConfig {
    pub server: ServerConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub quic_config: Option<QuicConfig>,
}

#[derive(Debug, Deserialize)]
pub struct QuicConfig {
    pub port: u16,
}
