use std::net::{SocketAddr, ToSocketAddrs};

use anyhow::{Context, anyhow};
use async_trait::async_trait;
use config::server::ProtocolConfig;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};
use tokio_rustls::{TlsAcceptor, TlsConnector};

use crate::{MaybeTlsStream, Transport, load_client_tls_acceptor, load_server_tls_acceptor};

#[derive(Default)]
pub struct TcpTransport {
    no_delay: bool,
    tls_acceptor: Option<TlsAcceptor>,
    tls_connector: Option<TlsConnector>,
}

#[derive(Default)]
pub struct TcpTransportClientOption {
    hostname: Option<String>,
}

pub struct TcpTransportServerOption {}

#[async_trait]
impl Transport for TcpTransport {
    type Listener = TcpListener;
    type RawConnection = MaybeTlsStream<TcpStream>;
    type Connection = net_mux::Session<MaybeTlsStream<TcpStream>>;
    type Channel = net_mux::Stream;
    type TransportClientOption = TcpTransportClientOption;
    type TransportServerOption = TcpTransportServerOption;

    fn new_server(
        config: &config::server::TransportConfig,
    ) -> anyhow::Result<(Self, Self::TransportServerOption)> {
        let ProtocolConfig::Tcp(tcp_config) = &config.protocol else {
            return Err(anyhow!("Invalid protocol config"));
        };
        Ok((
            Self {
                no_delay: tcp_config.no_delay,
                tls_acceptor: load_server_tls_acceptor(&tcp_config.tls)?,
                tls_connector: None,
            },
            Self::TransportServerOption {},
        ))
    }

    fn new_client(
        config: &config::client::TransportConfig,
    ) -> anyhow::Result<(Self, Self::TransportClientOption)> {
        let no_delay = config.transport_params.no_delay.unwrap_or(true);
        let hostname = config.transport_params.hostname.clone();
        Ok((
            Self {
                no_delay,
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
        TcpListener::bind(addr).await.map_err(anyhow::Error::from)
    }

    async fn accept(
        &self,
        l: &mut Self::Listener,
    ) -> anyhow::Result<(Self::RawConnection, SocketAddr)> {
        let (raw_stream, addr) = l.accept().await?;
        if self.no_delay {
            raw_stream.set_nodelay(true)?;
        }
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
        let raw_stream = TcpStream::connect(addr).await?;
        if self.no_delay {
            raw_stream.set_nodelay(true)?;
        }
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
