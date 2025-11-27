use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ProxyConfig {
    pub allowed_ports: Option<Vec<PortRangeConfig>>,

    pub tcp: Option<TcpProxyConfig>,
    pub http: Option<HttpProxyConfig>,
}

#[derive(Debug, Deserialize)]
pub enum PortRangeConfig {
    #[serde(rename = "single")]
    Single(u16),
    #[serde(rename = "range")]
    Range(u16, u16),
}

#[derive(Debug, Deserialize)]
pub struct TcpProxyConfig {}

#[derive(Debug, Deserialize)]
pub struct HttpProxyConfig {
    pub base_domain: String,
}
