mod http;
mod tcp;

use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};
use transport::Transport;

use crate::proxy::{http::HttpProxy, tcp::TcpProxy};

#[async_trait]
pub trait Proxy: Send + Sync {
    type Stream: AsyncRead + AsyncWrite + Send;

    fn from_config(config: &config::client::ProxyConfig) -> Result<Self>
    where
        Self: Sized;

    async fn handle(&self, stream: Self::Stream, local_addr: SocketAddr) -> Result<()>;
}

pub fn from_config<T: Transport + 'static>(
    config: &config::client::ProxyConfig,
) -> Result<Arc<dyn Proxy<Stream = T::Channel>>> {
    let proxy: Arc<dyn Proxy<Stream = T::Channel>> = match config.proxy_type {
        config::client::ProxyType::Tcp => Arc::new(TcpProxy::<T>::from_config(config)?),
        config::client::ProxyType::Http => Arc::new(HttpProxy::<T>::from_config(config)?),
    };
    Ok(proxy)
}
