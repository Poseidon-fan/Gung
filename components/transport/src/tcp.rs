use std::net::{SocketAddr, ToSocketAddrs};

use anyhow::anyhow;
use async_trait::async_trait;
use config::server::{ProtocolConfig, TransportConfig};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};

use crate::{LogicConnection, Transport};

pub struct TcpTransport {
    pub no_delay: bool,
}

pub struct TcpTransportClientOption {}

pub struct TcpTransportServerOption {}

#[async_trait]
impl Transport for TcpTransport {
    type Listener = TcpListener;
    type RawConnection = TcpStream;
    type Connection = net_mux::Session<TcpStream>;
    type Channel = net_mux::Stream;
    type TransportClientOption = TcpTransportClientOption;
    type TransportServerOption = TcpTransportServerOption;

    fn new_server(config: &TransportConfig) -> anyhow::Result<(Self, Self::TransportServerOption)> {
        let ProtocolConfig::Tcp(tcp_config) = &config.protocol else {
            return Err(anyhow!("Invalid protocol config"));
        };
        Ok((
            Self {
                no_delay: tcp_config.no_delay,
            },
            Self::TransportServerOption {},
        ))
    }

    fn new_client(
        config: &config::client::TransportConfig,
    ) -> anyhow::Result<(Self, Self::TransportClientOption)> {
        let no_delay = match config.transport_params.tcp_params {
            Some(ref tcp_param) => tcp_param.no_delay,
            None => false,
        };
        Ok((Self { no_delay }, Self::TransportClientOption {}))
    }

    async fn bind<T: ToSocketAddrs + Send>(
        &self,
        addr: T,
        _option: Self::TransportServerOption,
    ) -> anyhow::Result<Self::Listener> {
        let addr = addr.to_socket_addrs()?.next().unwrap();
        TcpListener::bind(addr).await.map_err(anyhow::Error::from)
    }

    async fn accept(
        &self,
        l: &Self::Listener,
    ) -> anyhow::Result<(Self::RawConnection, SocketAddr)> {
        l.accept()
            .await
            .map_err(anyhow::Error::from)
            .and_then(|(stream, addr)| {
                if self.no_delay {
                    stream.set_nodelay(true).map_err(anyhow::Error::from)?;
                }
                Ok((stream, addr))
            })
    }

    async fn connect<T: ToSocketAddrs + Send>(
        &self,
        addr: T,
        _option: Self::TransportClientOption,
    ) -> anyhow::Result<Self::RawConnection> {
        let addr = addr.to_socket_addrs()?.next().unwrap();
        TcpStream::connect(addr)
            .await
            .map_err(anyhow::Error::from)
            .and_then(|stream| {
                if self.no_delay {
                    stream.set_nodelay(true).map_err(anyhow::Error::from)?;
                }
                Ok(stream)
            })
    }

    fn establish(
        &self,
        raw_conn: Self::RawConnection,
        is_server: bool,
    ) -> anyhow::Result<Self::Connection> {
        // TODO(Poseidon): make here configurable
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

#[async_trait]
impl LogicConnection for net_mux::Session<TcpStream> {
    type Stream = net_mux::Stream;

    async fn accept(&self) -> anyhow::Result<Self::Stream> {
        self.accept().await.map_err(anyhow::Error::from)
    }

    async fn open(&self) -> anyhow::Result<Self::Stream> {
        self.open().await.map_err(anyhow::Error::from)
    }
}
