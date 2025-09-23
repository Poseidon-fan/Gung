mod quic;

use anyhow::Result;

use async_trait::async_trait;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{ToSocketAddrs, unix::SocketAddr},
};

#[async_trait]
pub trait Transport: Send + Sync {
    type Listener: Send + Sync;
    type Stream: Send + Sync + AsyncRead + AsyncWrite;

    async fn bind<T: ToSocketAddrs + Send + Sync>(&self, addr: T) -> Result<Self::Listener>;
    async fn accept(&self, l: &mut Self::Listener) -> Result<(Self::Stream, SocketAddr)>;
    async fn connect<T: ToSocketAddrs + Send + Sync>(&self, addr: T) -> Result<Self::Stream>;
    async fn close(&self, l: Self::Listener);
}
