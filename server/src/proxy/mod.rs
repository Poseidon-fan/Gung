#![allow(dead_code)]
pub mod tcp;

use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use anyhow::Result;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use protocol::{ClientCommandCodec, ServerCommand, ServerCommandCodec};
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
    select,
    sync::{mpsc, oneshot},
};
use tokio_util::codec::{FramedRead, FramedWrite};
use transport::LogicConnection;

use crate::{port::Port, proxy::tcp::TcpGateway};

#[async_trait]
pub trait Proxy: 'static + Sized + Sync + Send {
    type Request: Send;

    fn from_client_config(config: &config::client::ProxyConfig) -> Result<Self>;

    async fn handle_one<T>(&self, req: Self::Request, channel: T)
    where
        T: AsyncRead + AsyncWrite + Send + Unpin;

    async fn run<T: LogicConnection>(
        self,
        proxy_id: String,
        server_addr: SocketAddr,
        mut req_rx: mpsc::UnboundedReceiver<Self::Request>,
        conn: T,
        client_shutdown_tx: mpsc::UnboundedSender<String>,
        mut server_shutdown_rx: oneshot::Receiver<()>,
    ) -> Result<()> {
        // Wrap as Arc
        let proxy = Arc::new(self);

        // Request for a control channel
        let ctl_channel = conn.open().await?;
        let (client_cmd_reader, server_cmd_writer) = io::split(ctl_channel);
        let mut client_cmd_reader = FramedRead::new(client_cmd_reader, ClientCommandCodec);
        let mut server_cmd_writer = FramedWrite::new(server_cmd_writer, ServerCommandCodec);
        server_cmd_writer
            .send(ServerCommand::ForwardingStarted(server_addr))
            .await?;

        loop {
            select! {
                Some(req) = req_rx.recv() => {
                    let this = proxy.clone();
                    let data_channel = conn.open().await?;
                    tokio::spawn(async move {
                        this.handle_one(req, data_channel).await;
                    });
                },
                client_cmd = client_cmd_reader.next() => {
                    match client_cmd {
                        None | Some(Err(_)) => {
                            let _ = client_shutdown_tx.send(proxy_id.clone());
                            return Ok(());
                        }
                        Some(Ok(client_cmd)) => {
                            match client_cmd {
                                // TODO(Poseidon): implement client commands
                            }
                        }
                    }
                }
                _ = &mut server_shutdown_rx => {
                    let _ = client_shutdown_tx.send(proxy_id.clone());
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
            println!("get outside req");
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

type GatewayHandle<T> = (Arc<T>, Port);

pub struct GatewayManager<T: Gateway> {
    gateways: Arc<Mutex<HashMap<u16, GatewayHandle<T>>>>,
}

#[derive(Default)]
pub struct GatewayRegistry {
    pub tcp_mgr: Option<GatewayManager<TcpGateway>>,
}

impl<T: Gateway> GatewayManager<T> {
    pub fn new() -> Self {
        Self {
            gateways: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn register<K: LogicConnection + 'static>(
        &self,
        proxy: T::Proxy,
        proxy_id: String,
        port: Port,
        conn: K,
        client_shutdown_tx: mpsc::UnboundedSender<String>,
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
                    SocketAddr::from((Ipv4Addr::UNSPECIFIED, port.0)),
                    req_rx,
                    conn,
                    client_shutdown_tx,
                    server_shutdown_rx,
                )
                .await;
        });

        let exists = self
            .gateways
            .lock()
            .get(&port.0)
            .map(|(gtw, _)| gtw.clone());
        match exists {
            Some(gtw) => {
                gtw.add_proxy(pxy_handle);
            }
            None => {
                let gtw =
                    Arc::new(T::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, port.0))).await?);
                self.gateways.lock().insert(port.0, (gtw.clone(), port));
                gtw.add_proxy(pxy_handle);
                tokio::spawn(async move {
                    gtw.run().await;
                });
            }
        };

        Ok(())
    }
}

impl TryFrom<&config::server::ProxyConfig> for GatewayRegistry {
    type Error = anyhow::Error;

    fn try_from(config: &config::server::ProxyConfig) -> Result<Self> {
        let mut registry = Self::default();
        if let Some(_tcp_config) = config.tcp.as_ref() {
            registry.tcp_mgr = Some(GatewayManager::<TcpGateway>::new());
        }
        Ok(registry)
    }
}
