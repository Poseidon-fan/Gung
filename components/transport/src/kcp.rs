use std::{
    net::{SocketAddr, ToSocketAddrs},
    time::Duration,
};

use anyhow::anyhow;
use async_trait::async_trait;
use config::server::{ProtocolConfig, TransportConfig};
use tokio::io::AsyncWriteExt;
use tokio_kcp::{KcpConfig, KcpListener, KcpNoDelayConfig, KcpStream};

use crate::Transport;

pub struct KcpTransport {
    kcp_config: KcpConfig,
}

pub struct KcpTransportClientOption {}

pub struct KcpTransportServerOption {}

#[async_trait]
impl Transport for KcpTransport {
    type Listener = KcpListener;
    type RawConnection = KcpStream;
    type Connection = net_mux::Session<KcpStream>;
    type Channel = net_mux::Stream;
    type TransportClientOption = KcpTransportClientOption;
    type TransportServerOption = KcpTransportServerOption;

    fn new_server(config: &TransportConfig) -> anyhow::Result<(Self, Self::TransportServerOption)> {
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
            },
            Self::TransportServerOption {},
        ))
    }

    fn new_client(
        config: &config::client::TransportConfig,
    ) -> anyhow::Result<(Self, Self::TransportClientOption)> {
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
            },
            Self::TransportClientOption {},
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
        l.accept().await.map_err(anyhow::Error::from)
    }

    async fn connect<T: ToSocketAddrs + Send>(
        &self,
        addr: T,
        _option: Self::TransportClientOption,
    ) -> anyhow::Result<Self::RawConnection> {
        let addr = addr.to_socket_addrs()?.next().unwrap();
        KcpStream::connect(&self.kcp_config, addr)
            .await
            .map_err(anyhow::Error::from)
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
