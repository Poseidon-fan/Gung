mod kcp;
mod quic;
mod tcp;
mod websocket;

pub use kcp::*;
pub use quic::*;
use rustls::{RootCertStore, pki_types::ServerName};
pub use tcp::*;
use tokio_rustls::{TlsAcceptor, TlsConnector, TlsStream};
pub use websocket::*;

use std::{
    net::{SocketAddr, ToSocketAddrs},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use anyhow::Context as AnyhowContext;
use anyhow::Result;
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

// The abstraction of the transport layer.
#[async_trait]
pub trait Transport: Send + Sync {
    type Listener: Send + Sync;
    type RawConnection: Send + Sync + AsyncRead + AsyncWrite + Unpin + 'static;
    type Connection: LogicConnection<Stream = Self::Channel>;
    type Channel: Send + Sync + AsyncRead + AsyncWrite + Unpin;
    type TransportClientOption;
    type TransportServerOption;

    fn new_server(
        config: &config::server::TransportConfig,
    ) -> Result<(Self, Self::TransportServerOption)>
    where
        Self: Sized;

    fn new_client(
        config: &config::client::TransportConfig,
    ) -> Result<(Self, Self::TransportClientOption)>
    where
        Self: Sized;

    async fn bind<T: ToSocketAddrs + Send>(
        &self,
        addr: T,
        option: Self::TransportServerOption,
    ) -> Result<Self::Listener>;

    async fn accept(&self, l: &mut Self::Listener) -> Result<(Self::RawConnection, SocketAddr)>;

    async fn connect<T: ToSocketAddrs + Send>(
        &self,
        addr: T,
        option: Self::TransportClientOption,
    ) -> Result<Self::RawConnection>;

    fn establish(&self, raw_conn: Self::RawConnection, is_server: bool)
    -> Result<Self::Connection>;

    async fn abolish(&self, raw_conn: Self::RawConnection);
}

// The `LogicConnection` stands for a eeliable multiplexed long connection,
// it could be multiplexed into many `Stream`s that implement the `AsyncRead` and `AsyncWrite` traits.
#[async_trait]
pub trait LogicConnection: Send + Sync {
    type Stream: Send + Sync + AsyncRead + AsyncWrite + Unpin + 'static;

    async fn accept(&self) -> Result<Self::Stream>;

    async fn open(&self) -> Result<Self::Stream>;
}

#[async_trait]
impl<T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static> LogicConnection
    for net_mux::Session<T>
{
    type Stream = net_mux::Stream;

    async fn accept(&self) -> anyhow::Result<Self::Stream> {
        self.accept().await.map_err(anyhow::Error::from)
    }

    async fn open(&self) -> anyhow::Result<Self::Stream> {
        self.open().await.map_err(anyhow::Error::from)
    }
}

// Represent a stream that may be wrapped by TLS.
// Expose unified `AsyncRead` and `AsyncWrite` interface.
pub enum MaybeTlsStream<T: AsyncRead + AsyncWrite + Unpin + 'static> {
    Insecure(Box<T>),
    Secure(Box<TlsStream<T>>),
}

impl<T: AsyncRead + AsyncWrite + Unpin + 'static> MaybeTlsStream<T> {
    // Construct a server stream.
    pub async fn server(stream: T, acceptor: &Option<TlsAcceptor>) -> Result<Self> {
        match acceptor {
            Some(acceptor) => acceptor
                .accept(stream)
                .await
                .with_context(|| "tls handshake failed")
                .map(|stream| Self::Secure(Box::new(stream.into()))),
            None => Ok(Self::Insecure(Box::new(stream))),
        }
    }

    // Construct a client stream.
    pub async fn client(
        stream: T,
        connector: &Option<TlsConnector>,
        hostname: &str,
    ) -> Result<Self> {
        match connector {
            Some(connector) => {
                let domain = ServerName::try_from(hostname.to_string())?;
                connector
                    .connect(domain, stream)
                    .await
                    .with_context(|| "tls handshake failed")
                    .map(|stream| Self::Secure(Box::new(stream.into())))
            }
            None => Ok(Self::Insecure(Box::new(stream))),
        }
    }
}

impl<T: AsyncRead + AsyncWrite + Unpin + 'static> AsyncRead for MaybeTlsStream<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            MaybeTlsStream::Insecure(stream) => Pin::new(stream).poll_read(cx, buf),
            MaybeTlsStream::Secure(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl<T: AsyncRead + AsyncWrite + Unpin + 'static> AsyncWrite for MaybeTlsStream<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::result::Result<usize, std::io::Error>> {
        match self.get_mut() {
            MaybeTlsStream::Insecure(stream) => Pin::new(stream).poll_write(cx, buf),
            MaybeTlsStream::Secure(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), std::io::Error>> {
        match self.get_mut() {
            MaybeTlsStream::Insecure(stream) => Pin::new(stream).poll_flush(cx),
            MaybeTlsStream::Secure(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), std::io::Error>> {
        match self.get_mut() {
            MaybeTlsStream::Insecure(stream) => Pin::new(stream).poll_shutdown(cx),
            MaybeTlsStream::Secure(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

// Load TLS acceptor from server config.
pub fn load_server_tls_acceptor(
    tls_config: &Option<config::server::TlsConfig>,
) -> Result<Option<TlsAcceptor>> {
    match tls_config {
        Some(tls_config) => {
            let (cert, key) = cert::load_cert_key(&tls_config.key, &tls_config.cert)?;
            let server_config = tokio_rustls::rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(cert, key)?;
            Ok(Some(TlsAcceptor::from(Arc::new(server_config))))
        }
        None => Ok(None),
    }
}

// Load TLS acceptor from client config.
pub fn load_client_tls_acceptor(
    transport_params: &config::client::TransportParams,
) -> Result<Option<TlsConnector>> {
    match transport_params.no_tls {
        false => {
            // Load native certs.
            let mut roots = RootCertStore::empty();
            rustls_native_certs::load_native_certs()
                .expect("Could not load platform certs")
                .into_iter()
                .for_each(|cert| {
                    roots.add(cert).unwrap();
                });
            // Load custom certs if provided.
            transport_params
                .cert_path
                .as_ref()
                .map(|path| cert::load_certs(path))
                .transpose()?
                .into_iter()
                .flatten()
                .for_each(|cert| {
                    roots.add(cert).unwrap();
                });

            let client_config = tokio_rustls::rustls::ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth();
            Ok(Some(TlsConnector::from(Arc::new(client_config))))
        }
        true => Ok(None),
    }
}
