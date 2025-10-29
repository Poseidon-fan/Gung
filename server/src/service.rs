use std::sync::Arc;

use anyhow::Result;

use transport::Transport;

use crate::{
    config::RunConfig,
    proxy::{GatewayManager, tcp::TcpGateway},
};

pub struct Service<T: Transport> {
    config: Arc<RunConfig>,

    tcp_gtw_mgr: GatewayManager<TcpGateway>,

    transport: Arc<T>,
}

impl<T: Transport> Service<T> {
    pub fn run(&mut self, _option: T::TransportServerOption) -> Result<()> {
        // let listener = self.transport.bind(addr, option);

        Ok(())
    }
}
