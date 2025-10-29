use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;

use auth::Authenticator;
use config::server::RunConfig;
use transport::Transport;

use crate::proxy::{GatewayManager, tcp::TcpGateway};

pub struct Service<T: Transport> {
    config: Arc<RunConfig>,
    authenticator: Arc<dyn Authenticator>,

    tcp_gtw_mgr: GatewayManager<TcpGateway>,

    transport: Arc<T>,
}

impl<T: Transport> Service<T> {
    pub fn from(config: RunConfig) -> Result<(Self, T::TransportServerOption)> {
        let config = Arc::new(config);
        let (transport, transport_option) = T::new_server(&config.transport)?;
        let authenticator = auth::from(&config.auth)?;
        Ok((
            Self {
                config,
                tcp_gtw_mgr: GatewayManager::new(),
                transport: Arc::new(transport),
                authenticator,
            },
            transport_option,
        ))
    }

    pub async fn run(&mut self, transport_option: T::TransportServerOption) -> Result<()> {
        let listener = self
            .transport
            .bind(self.config.transport.addr, transport_option)
            .await?;

        loop {
            let (raw_conn, remote_addr) = self.transport.accept(&listener).await?;
            let authenticator = self.authenticator.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection::<T>(raw_conn, remote_addr, authenticator).await {
                    eprintln!("Connection handling error from {}: {:?}", remote_addr, e);
                }
            });
        }
    }
}

#[allow(unused_variables)]
async fn handle_connection<T: Transport>(
    raw_conn: T::RawConnection,
    remote_addr: SocketAddr,
    authenticator: Arc<dyn Authenticator>,
) -> Result<()> {
    todo!()
}
