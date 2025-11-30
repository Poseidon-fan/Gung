mod port;
pub(crate) mod proxy;
mod service;

use anyhow::Result;
use pyo3::{append_to_inittab, prelude::*};

use config::server::{ProtocolConfig, RunConfig};
use transport::{KcpTransport, QuicTransport, TcpTransport, WebsocketTransport};

use crate::service::Service;

#[tokio::main]
pub async fn run_server(run_config: RunConfig) -> Result<()> {
    init(&run_config)?;
    match run_config.transport.protocol {
        ProtocolConfig::Quic(_) => {
            let (mut service, transport_option) = Service::<QuicTransport>::from(run_config)?;
            service.run(transport_option).await?;
        }
        ProtocolConfig::Tcp(_) => {
            let (mut service, transport_option) = Service::<TcpTransport>::from(run_config)?;
            service.run(transport_option).await?;
        }
        ProtocolConfig::Kcp(_) => {
            let (mut service, transport_option) = Service::<KcpTransport>::from(run_config)?;
            service.run(transport_option).await?;
        }
        ProtocolConfig::Websocket(_) => {
            let (mut service, transport_option) = Service::<WebsocketTransport>::from(run_config)?;
            service.run(transport_option).await?;
        }
    };

    Ok(())
}

fn init(config: &RunConfig) -> Result<()> {
    if config.plugin.python.is_some() {
        append_to_inittab!(gung);
    }
    plugin::init(&config.plugin)?;
    // TODO(Poseidon): support allowed ports
    port::init(&config.proxy.allowed_ports)?;
    Ok(())
}

// Declare here to avoid circular dependency
#[pymodule]
fn gung(m: &Bound<'_, PyModule>) -> PyResult<()> {
    auth::register_module(m)?;
    Ok(())
}
