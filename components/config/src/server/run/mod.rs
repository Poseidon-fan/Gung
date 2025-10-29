mod auth;
mod transport;

use serde::Deserialize;

pub use auth::*;
pub use transport::*;

#[derive(Debug, Deserialize)]
pub struct RunConfig {
    pub transport: TransportConfig,
    pub auth: AuthConfig,
}
