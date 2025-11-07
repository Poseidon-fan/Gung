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
    pub base_domain: String,
}
