use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ProxyConfig {
    pub tcp: Option<TcpProxyConfig>,
    pub http: Option<HttpProxyConfig>,
}

#[derive(Debug, Deserialize)]
pub struct TcpProxyConfig {}

#[derive(Debug, Deserialize)]
pub struct HttpProxyConfig {
    _base_domain: String,
}
