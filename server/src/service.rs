use std::{net::SocketAddr, sync::Arc};

use anyhow::{Result, bail};

use auth::{AuthContext, AuthReqCodec, AuthResp, AuthRespCodec, AuthType, Authenticator};
use config::server::RunConfig;
use futures_util::{SinkExt, StreamExt};
use semver::Version;
use tokio::io;
use tokio_util::codec::{FramedRead, FramedWrite};

use transport::Transport;

use crate::proxy::{GatewayManager, tcp::TcpGateway};

pub struct Service<T: Transport> {
    config: Arc<RunConfig>,
    authenticator: Arc<dyn Authenticator>,

    tcp_gtw_mgr: GatewayManager<TcpGateway>,

    transport: Arc<T>,
}

impl<T: Transport> Service<T> {
    pub fn from(config: RunConfig) -> Result<(Self, T::TransportServerOption)> {
        let config = Arc::new(config);
        let (transport, transport_option) = T::new_server(&config.transport)?;
        let authenticator = auth::from(&config.auth)?;
        Ok((
            Self {
                config,
                tcp_gtw_mgr: GatewayManager::new(),
                transport: Arc::new(transport),
                authenticator,
            },
            transport_option,
        ))
    }

    pub async fn run(&mut self, transport_option: T::TransportServerOption) -> Result<()> {
        let listener = self
            .transport
            .bind(self.config.transport.addr, transport_option)
            .await?;

        loop {
            let (raw_conn, remote_addr) = self.transport.accept(&listener).await?;
            let authenticator = self.authenticator.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection::<T>(raw_conn, remote_addr, authenticator).await {
                    eprintln!("Connection handling error from {}: {:?}", remote_addr, e);
                }
            });
        }
    }
}

async fn handle_connection<T: Transport>(
    mut raw_conn: T::RawConnection,
    remote_addr: SocketAddr,
    authenticator: Arc<dyn Authenticator>,
) -> Result<()> {
    // Authenticate the connection
    let (auth_type, _) = authenticate::<T>(&mut raw_conn, remote_addr, authenticator).await?;

    match auth_type {
        AuthType::Ping => {
            todo!()
        }
        AuthType::Connect => {
            todo!()
        }
    }
}

async fn authenticate<T: Transport>(
    raw_conn: &mut T::RawConnection,
    // TODO(Poseidon): may support banning addr
    _: SocketAddr,
    authenticator: Arc<dyn Authenticator>,
) -> Result<(AuthType, String)> {
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
    let auth_type = req.payload["auth_type"]
        .as_u64()
        .map(|v| v as u8)
        .ok_or(anyhow::anyhow!("auth_type is required"))?
        .try_into()?;

    let mut context = AuthContext::new(auth_type, version, req);

    loop {
        let resp = authenticator.authenticate(&context).await?;
        match &resp {
            AuthResp::Accept(_) => {
                resp_writer.send(resp).await?;
                return Ok((context.auth_type, context.auth_id.clone()));
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
