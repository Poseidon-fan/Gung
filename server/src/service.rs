use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result, anyhow, bail};

use auth::{AuthContext, AuthReqCodec, AuthResp, AuthRespCodec, Authenticator};
use auth::{PROXY_AUTH_FIELD, SERVER_IP_AUTH_FIELD, VERSION_AUTH_FIELD};
use config::server::RunConfig;
use futures_util::{SinkExt, StreamExt};
use semver::Version;
use tokio::{io, time};
use tokio_util::codec::{FramedRead, FramedWrite};

use tracing::{error, info, instrument, warn};
use transport::Transport;

use crate::proxy::{GatewayRegistry, Proxy, http::HttpProxy, tcp::TcpProxy};

pub struct Service<T: Transport> {
    config: Arc<RunConfig>,
    authenticator: Arc<dyn Authenticator>,

    gtw_mgrs: Arc<GatewayRegistry>,

    transport: Arc<T>,
}

impl<T: Transport + 'static> Service<T> {
    pub fn from(config: RunConfig) -> Result<(Self, T::TransportServerOption)> {
        let config = Arc::new(config);
        let (transport, transport_option) = T::new_server(&config.transport)?;
        let authenticator = auth::from(&config.auth)?;
        let gtw_mgrs = Arc::new(GatewayRegistry::try_from(&config.proxy)?);
        Ok((
            Self {
                config,
                transport: Arc::new(transport),
                authenticator,
                gtw_mgrs,
            },
            transport_option,
        ))
    }

    pub async fn run(&mut self, transport_option: T::TransportServerOption) -> Result<()> {
        let mut listener = self
            .transport
            .bind(self.config.transport.addr, transport_option)
            .await
            .with_context(|| format!("Failed to bind transport {}", self.config.transport.addr))?;
        info!("Listening on {}", self.config.transport.addr);

        loop {
            match self.transport.accept(&mut listener).await {
                Ok((raw_conn, client_addr)) => {
                    let authenticator = Arc::clone(&self.authenticator);
                    let transport = Arc::clone(&self.transport);
                    let config = Arc::clone(&self.config);
                    let gtw_mgrs = Arc::clone(&self.gtw_mgrs);
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection::<T>(
                            raw_conn,
                            client_addr,
                            config,
                            authenticator,
                            gtw_mgrs,
                            transport,
                        )
                        .await
                        {
                            error!("Connection handling error from {client_addr}: {e:?}");
                        }
                    });
                }
                Err(e) => {
                    warn!("Failed to accept connection, err: {e:?}");
                }
            }
        }
    }
}

#[instrument(skip_all, fields(client_addr = client_addr.to_string()))]
async fn handle_connection<T: Transport + 'static>(
    mut raw_conn: T::RawConnection,
    client_addr: SocketAddr,
    config: Arc<RunConfig>,
    authenticator: Arc<dyn Authenticator>,
    gtw_mgrs: Arc<GatewayRegistry>,
    transport: Arc<T>,
) -> Result<()> {
    // Authenticate the connection
    let (context, conn) = match authenticate::<T>(
        &mut raw_conn,
        client_addr,
        authenticator,
        config.auth.timeout,
    )
    .await
    {
        Ok(context) => {
            info!("Authenticated successfully");
            (context, transport.establish(raw_conn, true)?)
        }
        Err(e) => {
            info!("Authentication failed: {e}");
            transport.abolish(raw_conn).await;
            return Ok(());
        }
    };

    let pointed_port = context.proxy.proxy_params.remote_port;
    match context.proxy.proxy_type {
        config::client::ProxyType::Tcp => {
            let proxy = TcpProxy::from_client_config(&context.proxy)?;
            gtw_mgrs
                .as_ref()
                .tcp_mgr
                .as_ref()
                .ok_or(anyhow!("TCP manager not supported"))?
                .register(proxy, context, config, pointed_port, conn)
                .await?;
        }
        config::client::ProxyType::Http => {
            let proxy = HttpProxy::from_client_config(&context.proxy)?;
            gtw_mgrs
                .as_ref()
                .http_mgr
                .as_ref()
                .ok_or(anyhow!("HTTP manager not supported"))?
                .register(proxy, context, config, pointed_port, conn)
                .await?;
        }
    };
    Ok(())
}

async fn authenticate<T: Transport>(
    raw_conn: &mut T::RawConnection,
    client_addr: SocketAddr,
    authenticator: Arc<dyn Authenticator>,
    timeout: u64,
) -> Result<AuthContext> {
    let timeout = Duration::from_secs(timeout);
    let (req_reader, resp_writer) = io::split(raw_conn);
    let mut req_reader = FramedRead::new(req_reader, AuthReqCodec);
    let mut resp_writer = FramedWrite::new(resp_writer, AuthRespCodec);
    // Read the first request and construct the context
    let req = time::timeout(timeout, req_reader.next())
        .await
        .with_context(|| anyhow!("failed to read first request"))?
        .transpose()?
        .ok_or(anyhow!("failed to read first request"))?;
    let version = req.payload[VERSION_AUTH_FIELD]
        .as_str()
        .ok_or(anyhow!("version is required"))?
        .parse::<Version>()?;
    let proxy = serde_json::from_value(req.payload[PROXY_AUTH_FIELD].clone())?;
    let server_ip = req.payload[SERVER_IP_AUTH_FIELD]
        .as_str()
        .ok_or(anyhow!("server_ip is required"))?
        .parse::<IpAddr>()?;

    let mut context = AuthContext::new(version, server_ip, client_addr, req, proxy);

    loop {
        let resp = authenticator.authenticate(&context).await?;
        match &resp {
            AuthResp::Accept(_) => {
                resp_writer.send(resp).await?;
                return Ok(context);
            }
            AuthResp::Reject(_) => {
                resp_writer.send(resp).await?;
                bail!("authentication rejected");
            }
            AuthResp::Challenge(_) => {
                // TODO(Poseidon): maybe need not to clone the resp here ?
                resp_writer.send(resp.clone()).await?;
                context.responses.push(resp);
                context.requests.push(
                    time::timeout(timeout, req_reader.next())
                        .await
                        .with_context(|| anyhow!("read request timeout"))?
                        .transpose()?
                        .ok_or(anyhow!("read request timeout"))?,
                );
            }
        }
    }
}
