mod proxy;

use std::sync::Arc;
use std::{io::Write, time::Duration};

use auth::{
    AuthAcceptResp, AuthChallengeResp, AuthRejectResp, AuthReq, AuthReqCodec, AuthResp,
    AuthRespCodec,
};
use config::client::{CliConfig, TransportType};

use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use protocol::{
    cmd::{ClientCommand, ClientCommandCodec, ServerCommand, ServerCommandCodec},
    constant::{PROXY_AUTH_FIELD, SERVER_IP_AUTH_FIELD, VERSION_AUTH_FIELD},
};
use serde_json::{Map, Value as JsonValue};
use tokio::{
    io::{self, AsyncBufReadExt, BufReader},
    select, time,
};
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{debug, error, info, instrument};
use transport::{LogicConnection, QuicTransport, TcpTransport, Transport};

use crate::proxy::Proxy;

#[tokio::main]
pub async fn run_client(config: CliConfig) -> Result<()> {
    match config.transport.transport_type {
        TransportType::Quic => {
            let (mut client, transport_option) = Client::<QuicTransport>::new(config)?;
            client.run(transport_option).await?;
        }
        TransportType::Tcp => {
            let (mut client, transport_option) = Client::<TcpTransport>::new(config)?;
            client.run(transport_option).await?;
        }
    };
    Ok(())
}

struct Client<T: Transport> {
    config: CliConfig,
    transport: Arc<T>,
    proxy: Arc<dyn Proxy<Stream = T::Channel>>,
}

impl<T: Transport + 'static> Client<T> {
    pub fn new(config: CliConfig) -> Result<(Self, T::TransportClientOption)> {
        let (transport, transport_option) = T::new_client(&config.transport)?;
        let proxy = proxy::from_config::<T>(&config.proxy)?;
        Ok((
            Self {
                config,
                transport: Arc::new(transport),
                proxy,
            },
            transport_option,
        ))
    }

    pub async fn run(&mut self, transport_option: T::TransportClientOption) -> Result<()> {
        let raw_conn = self
            .transport
            .connect(
                self.config
                    .transport
                    .transport_params
                    .server_addr
                    .to_string(),
                transport_option,
            )
            .await
            .with_context(|| {
                format!(
                    "Failed to connect to server {}",
                    self.config.transport.transport_params.server_addr
                )
            })?;

        handle_connection::<T>(
            raw_conn,
            &self.config,
            Arc::clone(&self.transport),
            Arc::clone(&self.proxy),
        )
        .await
    }
}

#[instrument(skip_all)]
async fn handle_connection<T: Transport + 'static>(
    mut raw_conn: T::RawConnection,
    config: &CliConfig,
    transport: Arc<T>,
    proxy: Arc<dyn Proxy<Stream = T::Channel>>,
) -> Result<()> {
    if let Err(e) = authenticate::<T>(&mut raw_conn, config).await {
        error!("Authentication failed: {e}");
        transport.abolish(raw_conn).await;
        return Ok(());
    }

    info!("Authentication successfully");

    let conn = transport.establish(raw_conn, false)?;
    let ctl_channel = conn.accept().await?;
    let (server_cmd_reader, client_cmd_writer) = io::split(ctl_channel);
    let mut server_cmd_reader = FramedRead::new(server_cmd_reader, ServerCommandCodec);
    let mut client_cmd_writer = FramedWrite::new(client_cmd_writer, ClientCommandCodec);
    info!("Proxy started");

    loop {
        select! {
            data_channel = conn.accept() => {
                match data_channel {
                    Ok(data_channel) => {
                        let local_addr = config.local_addr;
                        let proxy = Arc::clone(&proxy);
                        tokio::spawn(async move {
                            info!("Forwarding new stream");
                            let _ = proxy.handle(data_channel, local_addr).await;
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept data channel, error: {e}");
                        break;
                    }
                }
            }

            server_cmd = server_cmd_reader.next() => {
                match server_cmd {
                    None => {
                        info!("Server closed");
                        break;
                    },
                    Some(Err(e)) => {
                        error!("Internal error: {e}");
                        break;
                    },
                    Some(Ok(server_cmd)) => {
                        match server_cmd {
                            ServerCommand::ForwardingStarted(server_addr) => {
                                info!("Start forwarding to {server_addr}");
                            }
                            ServerCommand::ForwardingShutdown => {
                                info!("Server side shutdown");
                                break;
                            }
                            ServerCommand::Ping => {
                                debug!("Pong");
                                client_cmd_writer.send(ClientCommand::Ack).await?;
                            }
                        }
                    }
                }
            }

            _ = time::sleep(Duration::from_secs(config.transport.keepalive_timeout)) => {
                info!("Ping timeout");
                break;
            }
        }
    }
    Ok(())
}

async fn authenticate<T: Transport>(
    raw_conn: &mut T::RawConnection,
    config: &CliConfig,
) -> Result<()> {
    let (resp_reader, req_writer) = io::split(raw_conn);
    let mut req_writer = FramedWrite::new(req_writer, AuthReqCodec);
    let mut resp_reader = FramedRead::new(resp_reader, AuthRespCodec);

    // Construct the first request
    let mut req = match &config.data {
        Some(data) => {
            if !data.is_object() {
                bail!("data must be an object");
            } else {
                AuthReq {
                    payload: data.clone(),
                }
            }
        }
        None => AuthReq {
            payload: serde_json::from_str("{}").unwrap(),
        },
    };
    req.payload.as_object_mut().unwrap().insert(
        VERSION_AUTH_FIELD.to_string(),
        JsonValue::String(env!("CARGO_PKG_VERSION").to_string()),
    );
    req.payload.as_object_mut().unwrap().insert(
        PROXY_AUTH_FIELD.to_string(),
        serde_json::to_value(config.proxy.clone())?,
    );
    req.payload.as_object_mut().unwrap().insert(
        SERVER_IP_AUTH_FIELD.to_string(),
        JsonValue::String(
            config
                .transport
                .transport_params
                .server_addr
                .ip()
                .to_string(),
        ),
    );
    req_writer.send(req).await?;
    loop {
        match resp_reader.next().await.transpose()? {
            Some(resp) => match resp {
                AuthResp::Accept(AuthAcceptResp { msg }) => {
                    info!("Authentication accepted: {}", msg);
                    return Ok(());
                }
                AuthResp::Reject(AuthRejectResp { msg }) => {
                    info!("Authentication rejected: {}", msg);
                    bail!("authentication rejected: {}", msg);
                }
                AuthResp::Challenge(AuthChallengeResp {
                    msg,
                    required_fields,
                }) => {
                    println!("Server is challenging: {msg}, require more auth information");
                    let mut new_req = JsonValue::Object(Map::new());
                    let mut reader = BufReader::new(io::stdin());
                    for required_field in required_fields {
                        print!("{}: ", required_field);
                        std::io::stdout().flush()?;
                        let mut line = String::new();
                        reader.read_line(&mut line).await?;
                        let line = line.trim().to_string();
                        new_req
                            .as_object_mut()
                            .unwrap()
                            .insert(required_field, serde_json::from_str(line.as_str())?);
                    }
                    req_writer.send(AuthReq { payload: new_req }).await?;
                }
            },
            None => {
                bail!("failed to read response from peer");
            }
        }
    }
}
