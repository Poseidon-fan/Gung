mod transport;

use serde::Deserialize;

pub use transport::*;

#[derive(Debug, Deserialize)]
pub struct RunConfig {
    pub transport: TransportConfig,
}
