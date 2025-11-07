use std::net::SocketAddr;

use anyhow::Result;
use async_trait::async_trait;
use parking_lot::Mutex;
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
};
use tracing::debug;

use crate::proxy::{Gateway, Proxy, ProxyHandle};

pub struct TcpProxy {}

pub struct TcpGateway {
    listener: TcpListener,
    proxy_handle: Mutex<Option<ProxyHandle<TcpProxy>>>,
}

#[async_trait]
impl Proxy for TcpProxy {
    type Request = TcpStream;

    fn from_client_config(_config: &config::client::ProxyConfig) -> Result<Self> {
        Ok(Self {})
    }

    async fn handle_one<T>(&self, mut req: Self::Request, mut channel: T)
    where
        T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        debug!("start forwarding tcp");
        let _ = io::copy_bidirectional(&mut req, &mut channel).await;
        debug!("finish forwarding tcp");
    }
}

#[async_trait]
impl Gateway for TcpGateway {
    type RawStream = TcpStream;
    type Proxy = TcpProxy;

    async fn bind(addr: SocketAddr) -> Result<Self> {
        TcpListener::bind(addr)
            .await
            .map_err(anyhow::Error::from)
            .map(|listener| Self {
                listener,
                proxy_handle: Mutex::new(None),
            })
    }

    async fn accept(&self) -> Result<(Self::RawStream, SocketAddr)> {
        self.listener.accept().await.map_err(anyhow::Error::from)
    }

    async fn dispatch(&self, stream: Self::RawStream) {
        let _ = self
            .proxy_handle
            .lock()
            .as_ref()
            .unwrap()
            .req_tx
            .send(stream);
    }

    fn add_proxy(&self, handle: ProxyHandle<Self::Proxy>, _config: &config::client::ProxyConfig) {
        *self.proxy_handle.lock() = Some(handle);
    }

    fn remove_proxy(&self, proxy_id: String) {
        let handle = self.proxy_handle.lock().take();
        if let Some(handle) = handle {
            debug_assert_eq!(handle.proxy_id, proxy_id);
            let _ = handle.server_shutdown_tx.send(());
        }
    }

    fn is_empty(&self) -> bool {
        self.proxy_handle.lock().is_none()
    }
}
