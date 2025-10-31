mod auth;
mod proxy;
mod transport;

use serde::Deserialize;

pub use auth::*;
pub use proxy::*;
pub use transport::*;

#[derive(Debug, Deserialize)]
pub struct RunConfig {
    pub transport: TransportConfig,
    pub auth: AuthConfig,
    pub proxy: ProxyConfig,
}
