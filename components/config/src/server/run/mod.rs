mod transport;

use serde::Deserialize;

use crate::server::run::transport::TransportConfig;

#[derive(Debug, Deserialize)]
pub struct RunConfig {
    pub transport: TransportConfig,
}
