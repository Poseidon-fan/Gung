use std::{marker::PhantomData, net::SocketAddr};

use anyhow::Result;
use async_trait::async_trait;
use transport::Transport;

use crate::proxy::Proxy;

pub struct HttpProxy<T: Transport> {
    _phantom: PhantomData<T>,
}

#[async_trait]
impl<T: Transport> Proxy for HttpProxy<T> {
    type Stream = T::Channel;

    fn from_config(_config: &config::client::ProxyConfig) -> Result<Self>
    where
        Self: Sized,
    {
        todo!()
    }
    async fn handle(&self, _stream: Self::Stream, _local_addr: SocketAddr) -> Result<()> {
        todo!()
    }
}
