use std::{marker::PhantomData, net::SocketAddr};

use anyhow::Result;
use async_trait::async_trait;
use tokio::{io, net::TcpStream};
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
        Ok(Self {
            _phantom: PhantomData,
        })
    }
    async fn handle(&self, mut stream: Self::Stream, local_addr: SocketAddr) -> Result<()> {
        let mut local_socket = TcpStream::connect(local_addr).await?;
        println!("start forwarding");
        let _ = io::copy_bidirectional(&mut stream, &mut local_socket).await;
        println!("finish forwarding");
        Ok(())
    }
}
