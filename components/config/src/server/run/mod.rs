mod auth;
mod plugin;
mod proxy;
mod transport;

use serde::Deserialize;

pub use auth::*;
pub use plugin::*;
pub use proxy::*;
pub use transport::*;

#[derive(Debug, Deserialize)]
pub struct RunConfig {
    pub transport: TransportConfig,
    pub auth: AuthConfig,
    pub proxy: ProxyConfig,
    pub plugin: Option<PluginConfig>,
}
