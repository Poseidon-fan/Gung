mod quic;
mod tcp;

pub use quic::*;
pub use tcp::*;

use std::net::{SocketAddr, ToSocketAddrs};

use anyhow::Result;
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

#[async_trait]
pub trait Transport: Send + Sync {
    type Listener: Send + Sync;
    type RawConnection: Send + Sync + AsyncRead + AsyncWrite + Unpin;
    type Connection: LogicConnection<Stream = Self::Channel>;
    type Channel: Send + Sync + AsyncRead + AsyncWrite + Unpin;
    type TransportClientOption;
    type TransportServerOption;

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
}

#[async_trait]
pub trait LogicConnection: Send + Sync {
    type Stream: Send + Sync + AsyncRead + AsyncWrite + Unpin + 'static;

    async fn accept(&self) -> Result<Self::Stream>;
    async fn open(&self) -> Result<Self::Stream>;
}
