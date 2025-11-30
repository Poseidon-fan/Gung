use std::net::{SocketAddr, ToSocketAddrs};

use async_trait::async_trait;
use config::server::TransportConfig;
use tokio::io::AsyncWriteExt;
use tokio_kcp::{KcpConfig, KcpListener, KcpStream};

use crate::Transport;

pub struct KcpTransport {}

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

    fn new_server(
        _config: &TransportConfig,
    ) -> anyhow::Result<(Self, Self::TransportServerOption)> {
        Ok((Self {}, Self::TransportServerOption {}))
    }

    fn new_client(
        _config: &config::client::TransportConfig,
    ) -> anyhow::Result<(Self, Self::TransportClientOption)> {
        Ok((Self {}, Self::TransportClientOption {}))
    }

    async fn bind<T: ToSocketAddrs + Send>(
        &self,
        addr: T,
        _option: Self::TransportServerOption,
    ) -> anyhow::Result<Self::Listener> {
        let addr = addr.to_socket_addrs()?.next().unwrap();
        let config = KcpConfig::default();
        KcpListener::bind(config, addr)
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
        let config = KcpConfig::default();
        KcpStream::connect(&config, addr)
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
