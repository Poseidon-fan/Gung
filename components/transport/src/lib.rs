mod option;
mod quic;

use std::net::{SocketAddr, ToSocketAddrs};

use anyhow::Result;
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

#[async_trait]
pub trait Transport: Send + Sync {
    type Listener: Send + Sync;
    type RawConnection: Send + Sync + AsyncRead + AsyncWrite;
    type Connection: LogicConnection<Stream = Self::Channel>;
    type Channel: Send + Sync + AsyncRead + AsyncWrite;

    type ServerOption;
    type ClientOption;

    async fn bind<T: ToSocketAddrs + Send>(
        addr: T,
        option: Self::ServerOption,
    ) -> Result<Self::Listener>;
    async fn accept(l: &mut Self::Listener) -> Result<(Self::RawConnection, SocketAddr)>;
    async fn connect<T: ToSocketAddrs + Send>(
        addr: T,
        option: Self::ClientOption,
    ) -> Result<Self::RawConnection>;
    fn establish(raw_conn: Self::RawConnection, is_server: bool) -> Result<Self::Connection>;
}

#[async_trait]
pub trait LogicConnection: Send + Sync {
    type Stream: Send + Sync + AsyncRead + AsyncWrite;

    async fn accept(&self) -> Result<Self::Stream>;
    async fn open(&self) -> Result<Self::Stream>;
}
