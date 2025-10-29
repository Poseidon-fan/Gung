use std::sync::Arc;

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
        #[allow(unused_variables)]
        let listener = self
            .transport
            .bind(self.config.transport.addr, transport_option)
            .await?;

        loop {
            let (_, remote_addr) = self.transport.accept(&listener).await?;
            println!("accepted connection from {remote_addr}");
        }
    }
}
