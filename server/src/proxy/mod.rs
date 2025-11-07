pub mod http;
pub mod tcp;

use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use async_trait::async_trait;
use auth::AuthContext;
use config::server::{KeepaliveConfig, RunConfig};
use futures_util::{SinkExt, StreamExt};
use protocol::{ClientCommand, ClientCommandCodec, ServerCommand, ServerCommandCodec};
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
    select,
    sync::{Mutex, mpsc, oneshot},
    time,
};
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{debug, error, info, instrument, warn};
use transport::LogicConnection;

use crate::{
    port::{self, Port},
    proxy::{http::HttpGateway, tcp::TcpGateway},
};

// The upper abstraction of the proxy.
#[async_trait]
pub trait Proxy: 'static + Sized + Sync + Send {
    type Request: Send;

    fn from_client_config(config: &config::client::ProxyConfig) -> Result<Self>;

    // Handle one `Request`
    async fn handle_one<T>(&self, req: Self::Request, channel: T) -> Result<()>
    where
        T: AsyncRead + AsyncWrite + Send + Unpin + 'static;

    #[instrument(name = "proxy:run", skip_all, fields(proxy_id = params.proxy_id))]
    async fn run<T: LogicConnection>(self, mut params: ProxyStartupParams<Self, T>) -> Result<()> {
        // Wrap as Arc
        let proxy = Arc::new(self);

        // Request for a control channel
        let ctl_channel = params.conn.open().await?;
        let (client_cmd_reader, server_cmd_writer) = io::split(ctl_channel);
        let mut client_cmd_reader = FramedRead::new(client_cmd_reader, ClientCommandCodec);
        let mut server_cmd_writer = FramedWrite::new(server_cmd_writer, ServerCommandCodec);

        // Notify the client that the forwarding has started, pass the proxy address to the client.
        server_cmd_writer
            .send(ServerCommand::ForwardingStarted(params.pxy_addr))
            .await?;

        // Keepalive ticker, detect if the client is still alive periodically.
        let mut ping_ticker =
            time::interval(Duration::from_secs(params.keepalive.keepalive_interval));

        info!("Proxy started");

        loop {
            select! {
                // Receive a request from the `Gateway`, get a data channel and forward the request to the client.
                Some(req) = params.req_rx.recv() => {
                    let this = Arc::clone(&proxy);
                    let data_channel = params.conn.open().await?;
                    tokio::spawn(async move {
                        debug!("Forwarding new request");
                        if let Err(e) = this.handle_one(req, data_channel).await {
                            warn!("Failed to handle request: {}", e);
                        }
                    });
                }

                // Receive a command from the client.
                client_cmd = client_cmd_reader.next() => {
                    match client_cmd {
                        None | Some(Err(_)) => {
                            info!("Client side shutdown");
                            let _ = params.client_shutdown_tx.send(params.proxy_id.clone());
                            return Ok(());
                        }
                        Some(Ok(client_cmd)) => {
                            match client_cmd {
                                ClientCommand::ClientShutdown => {
                                    info!("Client side shutdown");
                                    let _ = params.client_shutdown_tx.send(params.proxy_id.clone());
                                    return Ok(());
                                }
                                ClientCommand::Ack => {}
                            }
                        }
                    }
                }

                // Keepalive ping.
                _ = ping_ticker.tick() => {
                    debug!("Ping");
                    server_cmd_writer.send(ServerCommand::Ping).await?;
                }

                // Server shutdown the proxy proactively.
                _ = &mut params.server_shutdown_rx => {
                    info!("Server side shutdown");
                    let _ = params.client_shutdown_tx.send(params.proxy_id.clone());
                    return Ok(());
                }

                // Keepalive timeout, the client is not even responding to the ping.
                _ = time::sleep(Duration::from_secs(params.keepalive.keepalive_timeout)) => {
                    info!("Ping timeout");
                    let _ = params.client_shutdown_tx.send(params.proxy_id.clone());
                    return Ok(());
                }
            }
        }
    }
}

struct ProxyStartupParams<P: Proxy, T: LogicConnection> {
    pub proxy_id: String,
    pub pxy_addr: String,
    pub keepalive: KeepaliveConfig,
    pub req_rx: mpsc::UnboundedReceiver<P::Request>,
    pub conn: T,
    pub client_shutdown_tx: mpsc::UnboundedSender<String>,
    pub server_shutdown_rx: oneshot::Receiver<()>,
}

pub struct ProxyHandle<T: Proxy> {
    proxy_id: String,
    // The channel to dispatch the `Request` to the `Proxy`.
    req_tx: mpsc::UnboundedSender<T::Request>,
    server_shutdown_tx: oneshot::Sender<()>,
}

// Gateway is responsible for managing the proxies as well as receiving the raw stream and dispatching it to the `Proxy`.
#[async_trait]
pub trait Gateway: 'static + Sized + Send + Sync {
    type RawStream: Send;
    type Proxy: Proxy;

    async fn bind(addr: SocketAddr, pxy_config: &config::server::ProxyConfig) -> Result<Self>;

    async fn accept(&self) -> Result<(Self::RawStream, SocketAddr)>;

    async fn dispatch(&self, stream: Self::RawStream) -> Result<()>;

    async fn run(
        self: Arc<Self>,
        port: u16,
        mut client_shutdown_rx: mpsc::UnboundedReceiver<String>,
        gateway_shutdown_tx: mpsc::UnboundedSender<u16>,
    ) {
        loop {
            select! {
                // Accept a new remote connection.
                acc = self.accept() => {
                    match acc {
                        Ok((raw_stream, _)) => {
                            let this = Arc::clone(&self);
                            tokio::spawn(async move {
                                if let Err(e) = this.dispatch(raw_stream).await {
                                    warn!("Failed to dispatch request: {}", e);
                                }
                            });
                        },
                        Err(_) => {
                            error!("Failed to accept connection");
                        }
                    }
                },
                // Receive a shutdown signal from the background task.
                Some(proxy_id) = client_shutdown_rx.recv() => {
                    self.remove_proxy(proxy_id);
                    if self.is_empty() {
                        // Release the port if there's no proxy left.
                        let _ = gateway_shutdown_tx.send(port);
                    }
                }
            }
        }
    }

    fn add_proxy(
        &self,
        handle: ProxyHandle<Self::Proxy>,
        port: u16,
        config: &config::client::ProxyConfig,
    ) -> Result<String>;

    fn remove_proxy(&self, proxy_id: String);

    fn is_empty(&self) -> bool;
}

struct GatewayHandle<T> {
    // Contain the `Port`'s ownership, so that the port will be freed automatically when the gateway is closed.
    _port: Port,
    gtw: Arc<T>,
    client_shutdown_tx: mpsc::UnboundedSender<String>,
}

pub struct GatewayManager<T: Gateway> {
    gateways: Arc<Mutex<HashMap<u16, GatewayHandle<T>>>>,
    gateway_shutdown_tx: mpsc::UnboundedSender<u16>,
}

#[derive(Default)]
pub struct GatewayRegistry {
    pub tcp_mgr: Option<GatewayManager<TcpGateway>>,
    pub http_mgr: Option<GatewayManager<HttpGateway>>,
}

impl<T: Gateway> GatewayManager<T> {
    pub fn new() -> Self {
        let (gateway_shutdown_tx, gateway_shutdown_rx) = mpsc::unbounded_channel();
        let gateways = Arc::new(Mutex::new(HashMap::new()));
        tokio::spawn(collect_gateway_shutdown(
            Arc::clone(&gateways),
            gateway_shutdown_rx,
        ));
        Self {
            gateways,
            gateway_shutdown_tx,
        }
    }

    // Register a proxy to the gateway manager. If there's no gateway listening on the port, a new gateway will be created.
    pub async fn register<K: LogicConnection + 'static>(
        &self,
        proxy: T::Proxy,
        auth_ctx: AuthContext,
        config: Arc<RunConfig>,
        pointed_port: Option<u16>,
        conn: K,
    ) -> Result<()> {
        let proxy_id = auth_ctx.auth_id.clone();
        let (req_tx, req_rx) = mpsc::unbounded_channel();
        let (server_shutdown_tx, server_shutdown_rx) = oneshot::channel();
        let pxy_handle: ProxyHandle<T::Proxy> = ProxyHandle {
            proxy_id: proxy_id.clone(),
            req_tx,
            server_shutdown_tx,
        };

        let (pxy_addr, client_shutdown_tx) = {
            let mut gateways = self.gateways.lock().await;

            // Try to use an existing gateway or create a new one.
            match pointed_port.and_then(|port| gateways.get(&port).map(|gateway| (port, gateway))) {
                Some((port, existing_gateway)) => {
                    // Use existing gateway.
                    let gtw = Arc::clone(&existing_gateway.gtw);
                    let client_shutdown_tx = existing_gateway.client_shutdown_tx.clone();
                    let pxy_addr = gtw.add_proxy(pxy_handle, port, &auth_ctx.proxy)?;
                    (pxy_addr, client_shutdown_tx)
                }
                None => {
                    // Allocate a port for the gateway, the port will be freed automatically when the proxy is closed.
                    let allocated_port = port::alloc(pointed_port)?;
                    let allocated_port_u16 = allocated_port.0;

                    let gtw = Arc::new(
                        T::bind(
                            SocketAddr::from((Ipv4Addr::UNSPECIFIED, allocated_port_u16)),
                            &config.proxy,
                        )
                        .await?,
                    );

                    let (client_shutdown_tx, client_shutdown_rx) = mpsc::unbounded_channel();

                    gateways.insert(
                        allocated_port_u16,
                        GatewayHandle {
                            gtw: Arc::clone(&gtw),
                            _port: allocated_port,
                            client_shutdown_tx: client_shutdown_tx.clone(),
                        },
                    );

                    let pxy_addr =
                        gtw.add_proxy(pxy_handle, allocated_port_u16, &auth_ctx.proxy)?;

                    // Start gateway runtime task.
                    let gateway_shutdown_tx = self.gateway_shutdown_tx.clone();
                    tokio::spawn(async move {
                        gtw.run(allocated_port_u16, client_shutdown_rx, gateway_shutdown_tx)
                            .await;
                    });

                    (pxy_addr, client_shutdown_tx)
                }
            }
        };

        // Start a background task to handle the proxy.
        tokio::spawn(async move {
            let params = ProxyStartupParams {
                proxy_id: proxy_id.clone(),
                pxy_addr,
                keepalive: config.transport.keepalive.clone(),
                req_rx,
                conn,
                client_shutdown_tx,
                server_shutdown_rx,
            };
            if let Err(e) = proxy.run(params).await {
                warn!("Error running proxy: {e}");
            }
        });

        Ok(())
    }
}

// A background task to collect the gateway shutdown signal.
async fn collect_gateway_shutdown<T: Gateway>(
    gateways: Arc<Mutex<HashMap<u16, GatewayHandle<T>>>>,
    mut gateway_shutdown_rx: mpsc::UnboundedReceiver<u16>,
) {
    while let Some(port) = gateway_shutdown_rx.recv().await {
        debug!("Removed gateway on port {port}");
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
        if let Some(_http_config) = config.http.as_ref() {
            registry.http_mgr = Some(GatewayManager::<HttpGateway>::new());
        }
        Ok(registry)
    }
}
