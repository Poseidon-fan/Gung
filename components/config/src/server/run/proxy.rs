use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ProxyConfig {
    pub tcp: Option<TcpProxyConfig>,
}

#[derive(Debug, Deserialize)]
pub struct TcpProxyConfig {}
