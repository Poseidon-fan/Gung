use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use anyhow::{Result, anyhow, bail};

use auth::{AuthContext, AuthReqCodec, AuthResp, AuthRespCodec, Authenticator};
use config::server::RunConfig;
use futures_util::{SinkExt, StreamExt};
use semver::Version;
use tokio::io;
use tokio_util::codec::{FramedRead, FramedWrite};

use transport::Transport;

use crate::{
    port,
    proxy::{GatewayRegistry, Proxy, tcp::TcpProxy},
};

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
        // TODO(Poseidon): support allowed ports
        port::init(None)?;

        let listener = self
            .transport
            .bind(self.config.transport.addr, transport_option)
            .await?;

        loop {
            if let Ok((raw_conn, remote_addr)) = self.transport.accept(&listener).await {
                let authenticator = self.authenticator.clone();
                let transport = self.transport.clone();
                let gtw_mgrs = self.gtw_mgrs.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection::<T>(
                        raw_conn,
                        remote_addr,
                        authenticator,
                        gtw_mgrs,
                        transport,
                    )
                    .await
                    {
                        eprintln!("Connection handling error from {}: {:?}", remote_addr, e);
                    }
                });
            } else {
                todo!("handle accept error");
            }
        }
    }
}

async fn handle_connection<T: Transport + 'static>(
    mut raw_conn: T::RawConnection,
    remote_addr: SocketAddr,
    authenticator: Arc<dyn Authenticator>,
    gtw_mgrs: Arc<GatewayRegistry>,
    transport: Arc<T>,
) -> Result<()> {
    // Authenticate the connection
    let context = authenticate::<T>(&mut raw_conn, remote_addr, authenticator).await?;

    let conn = transport.establish(raw_conn, true)?;
    match context.proxy.proxy_type {
        config::client::ProxyType::Tcp => {
            let proxy = TcpProxy::from_client_config(&context.proxy)?;
            gtw_mgrs
                .as_ref()
                .tcp_mgr
                .as_ref()
                .ok_or(anyhow!("TCP manager not supported"))?
                .register(
                    proxy,
                    context.auth_id.clone(),
                    context.server_ip,
                    port::alloc(context.proxy.proxy_params.remote_port)?,
                    conn,
                )
                .await?;
        }
        config::client::ProxyType::Http => {
            todo!()
        }
    };
    Ok(())
}

async fn authenticate<T: Transport>(
    raw_conn: &mut T::RawConnection,
    // TODO(Poseidon): may support banning addr
    _: SocketAddr,
    authenticator: Arc<dyn Authenticator>,
) -> Result<AuthContext> {
    let (req_reader, resp_writer) = io::split(raw_conn);
    let mut req_reader = FramedRead::new(req_reader, AuthReqCodec);
    let mut resp_writer = FramedWrite::new(resp_writer, AuthRespCodec);

    // Read the first request and construct the context
    let req = req_reader
        .next()
        .await
        .ok_or(anyhow::anyhow!("failed to read first request"))??;
    let version = req.payload["version"]
        .as_str()
        .ok_or(anyhow::anyhow!("version is required"))?
        .parse::<Version>()?;
    let proxy = serde_json::from_value(req.payload["proxy"].clone())?;
    let server_ip = req.payload["server_ip"]
        .as_str()
        .ok_or(anyhow::anyhow!("server_ip is required"))?
        .parse::<IpAddr>()?;

    let mut context = AuthContext::new(version, server_ip, req, proxy);

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
                    req_reader
                        .next()
                        .await
                        .ok_or(anyhow::anyhow!("failed to read next request"))??,
                );
            }
        }
    }
}
