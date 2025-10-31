use std::sync::Arc;

use auth::{
    AuthAcceptResp, AuthRejectResp, AuthReq, AuthReqCodec, AuthResp, AuthRespCodec, AuthType,
};
use config::client::{CliConfig, TransportType};

use anyhow::{Result, bail};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value as JsonValue;
use tokio::io;
use tokio_util::codec::{FramedRead, FramedWrite};
use transport::{QuicTransport, TcpTransport, Transport};

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
}

impl<T: Transport> Client<T> {
    pub fn new(config: CliConfig) -> Result<(Self, T::TransportClientOption)> {
        let (transport, transport_option) = T::new_client(&config.transport)?;
        Ok((
            Self {
                config,
                transport: Arc::new(transport),
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
            .await?;

        handle_connection::<T>(raw_conn, &self.config).await
    }
}

async fn handle_connection<T: Transport>(
    mut raw_conn: T::RawConnection,
    config: &CliConfig,
) -> Result<()> {
    authenticate::<T>(&mut raw_conn, config).await?;
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
        "version".to_string(),
        JsonValue::String(env!("CARGO_PKG_VERSION").to_string()),
    );
    req.payload.as_object_mut().unwrap().insert(
        "auth_type".to_string(),
        JsonValue::String(AuthType::Connect.to_string()),
    );
    req.payload.as_object_mut().unwrap().insert(
        "proxy".to_string(),
        serde_json::to_value(config.proxy.clone())?,
    );
    println!("req: {:?}", req);
    req_writer.send(req).await?;
    loop {
        match resp_reader.next().await.transpose()? {
            Some(resp) => match resp {
                AuthResp::Accept(AuthAcceptResp { msg }) => {
                    println!("Authentication accepted: {}", msg);
                    return Ok(());
                }
                AuthResp::Reject(AuthRejectResp { msg }) => {
                    println!("Authentication rejected: {}", msg);
                    bail!("authentication rejected: {}", msg);
                }
                AuthResp::Challenge(_) => {
                    todo!()
                }
            },
            None => {
                bail!("failed to read response from peer");
            }
        }
    }
}
