mod quic;
mod tcp;

pub use quic::*;
pub use tcp::*;

use std::net::{SocketAddr, ToSocketAddrs};

use anyhow::Result;
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

// The abstraction of the transport layer.
#[async_trait]
pub trait Transport: Send + Sync {
    type Listener: Send + Sync;
    type RawConnection: Send + Sync + AsyncRead + AsyncWrite + Unpin + 'static;
    type Connection: LogicConnection<Stream = Self::Channel>;
    type Channel: Send + Sync + AsyncRead + AsyncWrite + Unpin;
    type TransportClientOption;
    type TransportServerOption;

    fn new_server(
        config: &config::server::TransportConfig,
    ) -> Result<(Self, Self::TransportServerOption)>
    where
        Self: Sized;

    fn new_client(
        config: &config::client::TransportConfig,
    ) -> Result<(Self, Self::TransportClientOption)>
    where
        Self: Sized;

    async fn bind<T: ToSocketAddrs + Send>(
        &self,
        addr: T,
        option: Self::TransportServerOption,
    ) -> Result<Self::Listener>;

    async fn accept(&self, l: &Self::Listener) -> Result<(Self::RawConnection, SocketAddr)>;

    async fn connect<T: ToSocketAddrs + Send>(
        &self,
        addr: T,
        option: Self::TransportClientOption,
    ) -> Result<Self::RawConnection>;

    fn establish(&self, raw_conn: Self::RawConnection, is_server: bool)
    -> Result<Self::Connection>;

    async fn abolish(&self, raw_conn: Self::RawConnection);
}

// The `LogicConnection` stands for a eeliable multiplexed long connection,
// it could be multiplexed into many `Stream`s that implement the `AsyncRead` and `AsyncWrite` traits.
#[async_trait]
pub trait LogicConnection: Send + Sync {
    type Stream: Send + Sync + AsyncRead + AsyncWrite + Unpin + 'static;

    async fn accept(&self) -> Result<Self::Stream>;

    async fn open(&self) -> Result<Self::Stream>;
}

#[async_trait]
impl<T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static> LogicConnection
    for net_mux::Session<T>
{
    type Stream = net_mux::Stream;

    async fn accept(&self) -> anyhow::Result<Self::Stream> {
        self.accept().await.map_err(anyhow::Error::from)
    }

    async fn open(&self) -> anyhow::Result<Self::Stream> {
        self.open().await.map_err(anyhow::Error::from)
    }
}
