mod proxy;

use std::sync::Arc;

use auth::{AuthAcceptResp, AuthRejectResp, AuthReq, AuthReqCodec, AuthResp, AuthRespCodec};
use config::client::{CliConfig, TransportType};

use anyhow::{Result, bail};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value as JsonValue;
use tokio::io;
use tokio_util::codec::{FramedRead, FramedWrite};
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
            .await?;

        handle_connection::<T>(
            raw_conn,
            &self.config,
            self.transport.clone(),
            self.proxy.clone(),
        )
        .await
    }
}

async fn handle_connection<T: Transport + 'static>(
    mut raw_conn: T::RawConnection,
    config: &CliConfig,
    transport: Arc<T>,
    proxy: Arc<dyn Proxy<Stream = T::Channel>>,
) -> Result<()> {
    authenticate::<T>(&mut raw_conn, config).await?;

    let conn = transport.establish(raw_conn, false)?;
    let _ctl_channel = conn.accept().await?;

    while let Ok(data_channel) = conn.accept().await {
        println!("get data channel");
        let local_addr = config.local_addr;
        let proxy = proxy.clone();
        tokio::spawn(async move {
            let _ = proxy.handle(data_channel, local_addr).await;
        });
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
        "version".to_string(),
        JsonValue::String(env!("CARGO_PKG_VERSION").to_string()),
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
