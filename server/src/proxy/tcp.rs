use std::net::SocketAddr;

use anyhow::Result;
use async_trait::async_trait;
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
};

use crate::proxy::{Gateway, Proxy, ProxyHandle};

pub struct TcpProxy {}

pub struct TcpGateway {
    listener: TcpListener,
    proxy_handle: ProxyHandle<TcpProxy>,
}

#[async_trait]
impl Proxy for TcpProxy {
    type Request = TcpStream;

    async fn handle_one<T>(&self, mut req: Self::Request, mut channel: T)
    where
        T: AsyncRead + AsyncWrite + Send + Unpin,
    {
        let _ = io::copy_bidirectional(&mut req, &mut channel).await;
    }
}

#[async_trait]
impl Gateway for TcpGateway {
    type RawStream = TcpStream;
    type Proxy = TcpProxy;

    async fn accept(&self) -> Result<(Self::RawStream, SocketAddr)> {
        self.listener.accept().await.map_err(anyhow::Error::from)
    }

    async fn upgrade(raw_stream: Self::RawStream) -> Result<<Self::Proxy as Proxy>::Request> {
        Ok(raw_stream)
    }

    async fn dispatch(&self, req: <Self::Proxy as Proxy>::Request) {
        let _ = self.proxy_handle.req_tx.send(req);
    }
}
