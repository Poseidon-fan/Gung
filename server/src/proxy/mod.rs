pub mod tcp;

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use parking_lot::Mutex;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite},
    select,
    sync::{mpsc, oneshot},
};
use transport::LogicConnection;

#[async_trait]
pub trait Proxy: 'static + Sized + Sync + Send {
    type Request: Send;

    async fn handle_one<T>(&self, req: Self::Request, channel: T)
    where
        T: AsyncRead + AsyncWrite + Send + Unpin;

    async fn run<T: LogicConnection>(
        self,
        proxy_id: String,
        mut req_rx: mpsc::UnboundedReceiver<Self::Request>,
        conn: T,
        client_shutdown_tx: mpsc::Sender<String>,
        mut server_shutdown_rx: oneshot::Receiver<()>,
    ) -> Result<()> {
        // Wrap as Arc
        let proxy = Arc::new(self);

        // Request for a control channel
        let mut ctl_channel = conn.open().await?;

        loop {
            select! {
                Some(req) = req_rx.recv() => {
                    let this = proxy.clone();
                    let data_channel = conn.open().await?;
                    tokio::spawn(async move {
                        this.handle_one(req, data_channel).await;
                    });
                },
                _ = ctl_channel.read_f32() => {
                    // TODO(Poseidon): protocol system for control channel
                    let _ = client_shutdown_tx.send(proxy_id.clone()).await;
                    return Ok(());
                },
                _ = &mut server_shutdown_rx => {
                    return Ok(());
                }
            }
        }
    }
}

pub struct ProxyHandle<T: Proxy> {
    proxy_id: String,
    req_tx: mpsc::UnboundedSender<T::Request>,
    server_shutdown_tx: oneshot::Sender<()>,
}

#[async_trait]
pub trait Gateway: 'static + Sized + Send + Sync {
    type RawStream: Send;
    type Proxy: Proxy;

    async fn bind(addr: SocketAddr) -> Result<Self>;

    async fn accept(&self) -> Result<(Self::RawStream, SocketAddr)>;

    async fn upgrade(raw_stream: Self::RawStream) -> Result<<Self::Proxy as Proxy>::Request>;

    async fn dispatch(&self, req: <Self::Proxy as Proxy>::Request);

    async fn run(self: Arc<Self>) {
        while let Ok((raw_stream, _)) = self.accept().await {
            let this = self.clone();
            tokio::spawn(async move {
                let req = Self::upgrade(raw_stream).await.unwrap();
                this.dispatch(req).await;
            });
        }
    }

    fn add_proxy(&self, handle: ProxyHandle<Self::Proxy>);

    fn remove_proxy(&self, proxy_id: String);

    fn is_empty(&self) -> bool;
}

pub struct GatewayManager<T: Gateway> {
    gateways: Arc<Mutex<HashMap<SocketAddr, Arc<T>>>>,
}

impl<T: Gateway> GatewayManager<T> {
    pub fn new() -> Self {
        Self {
            gateways: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn register<K: LogicConnection + 'static>(
        &mut self,
        proxy: T::Proxy,
        proxy_id: String,
        addr: SocketAddr,
        conn: K,
        client_shutdown_tx: mpsc::Sender<String>,
    ) -> Result<()> {
        let (req_tx, req_rx) = mpsc::unbounded_channel();
        let (server_shutdown_tx, server_shutdown_rx) = oneshot::channel();
        let pxy_handle: ProxyHandle<T::Proxy> = ProxyHandle {
            proxy_id: proxy_id.clone(),
            req_tx,
            server_shutdown_tx,
        };
        tokio::spawn(async move {
            // TODO(Poseidon): handle error here
            let _ = proxy
                .run(
                    proxy_id.clone(),
                    req_rx,
                    conn,
                    client_shutdown_tx,
                    server_shutdown_rx,
                )
                .await;
        });

        let exists = self.gateways.lock().get(&addr).cloned();
        match exists {
            Some(gtw) => {
                gtw.add_proxy(pxy_handle);
            }
            None => {
                let gtw = Arc::new(T::bind(addr).await?);
                self.gateways.lock().insert(addr, gtw.clone());
                gtw.add_proxy(pxy_handle);
                tokio::spawn(async move {
                    gtw.run().await;
                });
            }
        };

        Ok(())
    }
}
