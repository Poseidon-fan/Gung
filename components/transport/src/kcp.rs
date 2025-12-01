use std::{
    net::{SocketAddr, ToSocketAddrs},
    time::Duration,
};

use anyhow::{Context, anyhow};
use async_trait::async_trait;
use config::server::ProtocolConfig;
use tokio::io::AsyncWriteExt;
use tokio_kcp::{KcpConfig, KcpListener, KcpNoDelayConfig, KcpStream};
use tokio_rustls::{TlsAcceptor, TlsConnector};

use crate::{MaybeTlsStream, Transport, load_client_tls_acceptor, load_server_tls_acceptor};

pub struct KcpTransport {
    kcp_config: KcpConfig,
    tls_acceptor: Option<TlsAcceptor>,
    tls_connector: Option<TlsConnector>,
}

pub struct KcpTransportClientOption {
    hostname: Option<String>,
}

pub struct KcpTransportServerOption {}

#[async_trait]
impl Transport for KcpTransport {
    type Listener = KcpListener;
    type RawConnection = MaybeTlsStream<KcpStream>;
    type Connection = net_mux::Session<MaybeTlsStream<KcpStream>>;
    type Channel = net_mux::Stream;
    type TransportClientOption = KcpTransportClientOption;
    type TransportServerOption = KcpTransportServerOption;

    fn new_server(
        config: &config::server::TransportConfig,
    ) -> anyhow::Result<(Self, Self::TransportServerOption)> {
        let ProtocolConfig::Kcp(kcp_transport_config) = &config.protocol else {
            return Err(anyhow!("Invalid protocol config"));
        };
        Ok((
            Self {
                kcp_config: KcpConfig {
                    nodelay: KcpNoDelayConfig {
                        nodelay: kcp_transport_config.no_delay,
                        ..KcpNoDelayConfig::default()
                    },
                    session_expire: Duration::MAX,
                    stream: true,
                    ..KcpConfig::default()
                },
                tls_acceptor: load_server_tls_acceptor(&kcp_transport_config.tls)?,
                tls_connector: None,
            },
            Self::TransportServerOption {},
        ))
    }

    fn new_client(
        config: &config::client::TransportConfig,
    ) -> anyhow::Result<(Self, Self::TransportClientOption)> {
        let hostname = config.transport_params.hostname.clone();
        Ok((
            Self {
                kcp_config: KcpConfig {
                    nodelay: KcpNoDelayConfig {
                        nodelay: config.transport_params.no_delay.unwrap_or(true),
                        ..KcpNoDelayConfig::default()
                    },
                    session_expire: Duration::MAX,
                    stream: true,
                    ..KcpConfig::default()
                },
                tls_connector: load_client_tls_acceptor(&config.transport_params)
                    .with_context(|| "tls failed")?,
                tls_acceptor: None,
            },
            Self::TransportClientOption { hostname },
        ))
    }

    async fn bind<T: ToSocketAddrs + Send>(
        &self,
        addr: T,
        _option: Self::TransportServerOption,
    ) -> anyhow::Result<Self::Listener> {
        let addr = addr.to_socket_addrs()?.next().unwrap();
        KcpListener::bind(self.kcp_config, addr)
            .await
            .map_err(anyhow::Error::from)
    }

    async fn accept(
        &self,
        l: &mut Self::Listener,
    ) -> anyhow::Result<(Self::RawConnection, SocketAddr)> {
        let (raw_stream, addr) = l.accept().await?;
        Ok((
            MaybeTlsStream::server(raw_stream, &self.tls_acceptor).await?,
            addr,
        ))
    }

    async fn connect<T: ToSocketAddrs + Send>(
        &self,
        addr: T,
        option: Self::TransportClientOption,
    ) -> anyhow::Result<Self::RawConnection> {
        let addr = addr.to_socket_addrs()?.next().unwrap();
        let default_hostname = addr.ip().to_string();
        let hostname = option
            .hostname
            .as_ref()
            .unwrap_or(&default_hostname)
            .to_string();
        let raw_stream = KcpStream::connect(&self.kcp_config, addr).await?;
        Ok(MaybeTlsStream::client(raw_stream, &self.tls_connector, &hostname).await?)
    }

    fn establish(
        &self,
        raw_conn: Self::RawConnection,
        is_server: bool,
    ) -> anyhow::Result<Self::Connection> {
        match is_server {
            true => Ok(net_mux::Session::server(
                raw_conn,
                net_mux::Config::default(),
            )),
            false => Ok(net_mux::Session::client(
                raw_conn,
                net_mux::Config::default(),
            )),
        }
    }

    async fn abolish(&self, mut raw_conn: Self::RawConnection) {
        let _ = raw_conn.shutdown().await;
    }
}
