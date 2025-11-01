mod port;
pub(crate) mod proxy;
mod service;

use anyhow::{Ok, Result};

use config::server::{ProtocolConfig, RunConfig};
use transport::{QuicTransport, TcpTransport};

use crate::service::Service;

#[tokio::main]
pub async fn run_server(run_config: RunConfig) -> Result<()> {
    println!("run_config: {run_config:?}");

    match run_config.transport.protocol {
        ProtocolConfig::Quic(_) => {
            let (mut service, transport_option) = Service::<QuicTransport>::from(run_config)?;
            service.run(transport_option).await?;
        }
        ProtocolConfig::Tcp(_) => {
            let (mut service, transport_option) = Service::<TcpTransport>::from(run_config)?;
            service.run(transport_option).await?;
        }
    };

    Ok(())
}
