pub mod tcp;

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use async_trait::async_trait;
use config::server::KeepaliveConfig;
use futures_util::{SinkExt, StreamExt};
use protocol::cmd::{ClientCommand, ClientCommandCodec, ServerCommand, ServerCommandCodec};
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
    select,
    sync::{Mutex, mpsc, oneshot},
    time,
};
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{debug, info, instrument};
use transport::LogicConnection;

use crate::{port::Port, proxy::tcp::TcpGateway};

#[async_trait]
pub trait Proxy: 'static + Sized + Sync + Send {
    type Request: Send;

    fn from_client_config(config: &config::client::ProxyConfig) -> Result<Self>;

    async fn handle_one<T>(&self, req: Self::Request, channel: T)
    where
        T: AsyncRead + AsyncWrite + Send + Unpin;

    #[instrument(name = "proxy:run", skip_all, fields(proxy_id = proxy_id))]
    async fn run<T: LogicConnection>(
        self,
        proxy_id: String,
        server_addr: SocketAddr,
        keepalive: KeepaliveConfig,
        mut req_rx: mpsc::UnboundedReceiver<Self::Request>,
        conn: T,
        (client_shutdown_tx, mut server_shutdown_rx): (
            mpsc::UnboundedSender<String>,
            oneshot::Receiver<()>,
        ),
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

        let mut ping_ticker = time::interval(Duration::from_secs(keepalive.keepalive_interval));

        info!("Proxy started");

        loop {
            select! {
                Some(req) = req_rx.recv() => {
                    let this = proxy.clone();
                    let data_channel = conn.open().await?;
                    tokio::spawn(async move {
                        debug!("Forwarding new request");
                        this.handle_one(req, data_channel).await;
                    });
                }

                client_cmd = client_cmd_reader.next() => {
                    match client_cmd {
                        None | Some(Err(_)) => {
                            info!("Client side shutdown");
                            let _ = client_shutdown_tx.send(proxy_id.clone());
                            return Ok(());
                        }
                        Some(Ok(client_cmd)) => {
                            match client_cmd {
                                ClientCommand::ClientShutdown => {
                                    info!("Client side shutdown");
                                    let _ = client_shutdown_tx.send(proxy_id.clone());
                                    return Ok(());
                                }
                                ClientCommand::Ack => {}
                            }
                        }
                    }
                }

                _ = ping_ticker.tick() => {
                    debug!("Ping");
                    server_cmd_writer.send(ServerCommand::Ping).await?;
                }

                _ = &mut server_shutdown_rx => {
                    info!("Server side shutdown");
                    let _ = client_shutdown_tx.send(proxy_id.clone());
                    return Ok(());
                }

                _ = time::sleep(Duration::from_secs(keepalive.keepalive_timeout)) => {
                    info!("Ping timeout");
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

    async fn run(
        self: Arc<Self>,
        port: u16,
        mut client_shutdown_rx: mpsc::UnboundedReceiver<String>,
        gateway_shutdown_tx: mpsc::UnboundedSender<u16>,
    ) {
        loop {
            select! {
                acc = self.accept() => {
                    match acc {
                        Ok((raw_stream, _)) => {
                            println!("get outside req");
                            let this = self.clone();
                            tokio::spawn(async move {
                                let req = Self::upgrade(raw_stream).await.unwrap();
                                this.dispatch(req).await;
                            });
                        },
                        Err(_) => {
                            todo!()
                        }
                    }
                },
                Some(proxy_id) = client_shutdown_rx.recv() => {
                    self.remove_proxy(proxy_id);
                    if self.is_empty() {
                        let _ = gateway_shutdown_tx.send(port);
                    }
                }
            }
        }
    }

    fn add_proxy(&self, handle: ProxyHandle<Self::Proxy>);

    fn remove_proxy(&self, proxy_id: String);

    fn is_empty(&self) -> bool;
}

struct GatewayHandle<T> {
    gtw: Arc<T>,
    _port: Port,
    client_shutdown_tx: mpsc::UnboundedSender<String>,
}

pub struct GatewayManager<T: Gateway> {
    gateways: Arc<Mutex<HashMap<u16, GatewayHandle<T>>>>,
    gateway_shutdown_tx: mpsc::UnboundedSender<u16>,
}

#[derive(Default)]
pub struct GatewayRegistry {
    pub tcp_mgr: Option<GatewayManager<TcpGateway>>,
}

impl<T: Gateway> GatewayManager<T> {
    pub fn new() -> Self {
        let (gateway_shutdown_tx, gateway_shutdown_rx) = mpsc::unbounded_channel();
        let gateways = Arc::new(Mutex::new(HashMap::new()));
        tokio::spawn(collect_gateway_shutdown(
            gateways.clone(),
            gateway_shutdown_rx,
        ));
        Self {
            gateways,
            gateway_shutdown_tx,
        }
    }

    pub async fn register<K: LogicConnection + 'static>(
        &self,
        proxy: T::Proxy,
        proxy_id: String,
        server_ip: IpAddr,
        keepalive: KeepaliveConfig,
        port: Port,
        conn: K,
    ) -> Result<()> {
        let port_u16 = port.0;
        let (req_tx, req_rx) = mpsc::unbounded_channel();
        let (server_shutdown_tx, server_shutdown_rx) = oneshot::channel();
        let pxy_handle: ProxyHandle<T::Proxy> = ProxyHandle {
            proxy_id: proxy_id.clone(),
            req_tx,
            server_shutdown_tx,
        };
        let client_shutdown_tx = {
            let mut gateways = self.gateways.lock().await;
            match gateways.get(&port_u16) {
                Some(handle) => {
                    let gtw = handle.gtw.clone();
                    let tx = handle.client_shutdown_tx.clone();
                    gtw.add_proxy(pxy_handle);
                    tx
                }
                None => {
                    let gtw =
                        Arc::new(T::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, port.0))).await?);
                    let (client_shutdown_tx, client_shutdown_rx) = mpsc::unbounded_channel();

                    gateways.insert(
                        port.0,
                        GatewayHandle {
                            gtw: gtw.clone(),
                            _port: port,
                            client_shutdown_tx: client_shutdown_tx.clone(),
                        },
                    );
                    gtw.add_proxy(pxy_handle);
                    let gateway_shutdown_tx = self.gateway_shutdown_tx.clone();
                    tokio::spawn(async move {
                        gtw.run(port_u16, client_shutdown_rx, gateway_shutdown_tx)
                            .await;
                    });
                    client_shutdown_tx
                }
            }
        };
        tokio::spawn(async move {
            // TODO(Poseidon): handle error here
            let _ = proxy
                .run(
                    proxy_id.clone(),
                    SocketAddr::from((server_ip, port_u16)),
                    keepalive,
                    req_rx,
                    conn,
                    (client_shutdown_tx, server_shutdown_rx),
                )
                .await;
        });

        Ok(())
    }
}

async fn collect_gateway_shutdown<T: Gateway>(
    gateways: Arc<Mutex<HashMap<u16, GatewayHandle<T>>>>,
    mut gateway_shutdown_rx: mpsc::UnboundedReceiver<u16>,
) {
    while let Some(port) = gateway_shutdown_rx.recv().await {
        println!("removed {port}");
        gateways.lock().await.remove(&port);
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
