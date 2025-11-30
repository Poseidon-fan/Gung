use std::{
    net::{SocketAddr, ToSocketAddrs},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use async_trait::async_trait;
use config::{client::QuicTransportParams, server::ProtocolConfig};
use quinn::{
    Endpoint,
    crypto::rustls::{QuicClientConfig, QuicServerConfig},
};
use rustls::{
    RootCertStore,
    pki_types::{CertificateDer, PrivateKeyDer},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::{LogicConnection, Transport};
use anyhow::anyhow;

const ALPN_GUNG: &[&[u8]] = &[b"gung"];

pub struct QuicTransport {}

pub struct QuicConnection(quinn::Connection);

pub struct QuicStream {
    sender: quinn::SendStream,
    receiver: quinn::RecvStream,
}

pub struct QuicRawConnection {
    conn: quinn::Connection,
    // Unlike `TcpStream` witch natively supports AsyncRead and AsyncWrite,
    // here use a quic stream to communicate before established logically.
    stream: QuicStream,
}

pub struct QuicTransportClientOption {
    pub cert: Option<Vec<CertificateDer<'static>>>,
    pub hostname: Option<String>,
}

pub struct QuicTransportServerOption {
    pub cert: Vec<CertificateDer<'static>>,
    pub key: PrivateKeyDer<'static>,
}

#[async_trait]
impl Transport for QuicTransport {
    type Listener = quinn::Endpoint;
    type RawConnection = QuicRawConnection;
    type Connection = QuicConnection;
    type Channel = QuicStream;
    type TransportClientOption = QuicTransportClientOption;
    type TransportServerOption = QuicTransportServerOption;

    fn new_server(
        config: &config::server::TransportConfig,
    ) -> anyhow::Result<(Self, Self::TransportServerOption)> {
        let ProtocolConfig::Quic(quic_config) = &config.protocol else {
            return Err(anyhow!("Invalid protocol config"));
        };
        let (cert, key) = cert::get_cert_key(&quic_config.tls_key, &quic_config.tls_cert)?;
        Ok((Self {}, QuicTransportServerOption { cert, key }))
    }

    fn new_client(
        config: &config::client::TransportConfig,
    ) -> anyhow::Result<(Self, Self::TransportClientOption)> {
        if let Some(QuicTransportParams {
            cert_path,
            hostname,
        }) = &config.transport_params.quic_params
        {
            Ok((
                Self {},
                QuicTransportClientOption {
                    cert: cert_path
                        .as_ref()
                        .map(|path| cert::load_certs(path))
                        .transpose()?,
                    hostname: hostname.clone(),
                },
            ))
        } else {
            Ok((
                Self {},
                QuicTransportClientOption {
                    cert: None,
                    hostname: None,
                },
            ))
        }
    }

    async fn bind<T: ToSocketAddrs + Send>(
        &self,
        addr: T,
        option: Self::TransportServerOption,
    ) -> anyhow::Result<Self::Listener> {
        let mut server_crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(option.cert, option.key)?;
        server_crypto.alpn_protocols = ALPN_GUNG.iter().map(|&x| x.into()).collect();
        let mut server_config =
            quinn::ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(server_crypto)?));
        let transport_config = Arc::get_mut(&mut server_config.transport).unwrap();
        transport_config.max_concurrent_uni_streams(0_u8.into());
        transport_config.max_idle_timeout(None);

        Ok(Endpoint::server(
            server_config,
            addr.to_socket_addrs()?.next().unwrap(),
        )?)
    }

    async fn accept(
        &self,
        l: &mut Self::Listener,
    ) -> anyhow::Result<(Self::RawConnection, SocketAddr)> {
        let connection = l
            .accept()
            .await
            .ok_or(anyhow!("Failed to accept connection"))?
            .await?;
        let remote_addr = connection.remote_address();
        let (sender, receiver) = connection.accept_bi().await.map_err(anyhow::Error::from)?;
        let stream = QuicStream { sender, receiver };
        Ok((
            QuicRawConnection {
                conn: connection,
                stream,
            },
            remote_addr,
        ))
    }

    async fn connect<T: ToSocketAddrs + Send>(
        &self,
        addr: T,
        option: Self::TransportClientOption,
    ) -> anyhow::Result<Self::RawConnection> {
        let socket_addr = addr.to_socket_addrs()?.next().unwrap();
        let default_hostname = socket_addr.ip().to_string();
        let hostname = option
            .hostname
            .as_ref()
            .unwrap_or(&default_hostname)
            .to_string();

        let mut roots = RootCertStore::empty();
        for cert in rustls_native_certs::load_native_certs().expect("Could not load platform certs")
        {
            roots.add(cert).unwrap();
        }
        if let Some(cert) = option.cert {
            for cert in cert {
                roots.add(cert).unwrap();
            }
        }
        let mut client_crypto = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        client_crypto.alpn_protocols = ALPN_GUNG.iter().map(|&x| x.into()).collect();
        let mut client_config =
            quinn::ClientConfig::new(Arc::new(QuicClientConfig::try_from(client_crypto)?));

        let mut transport_config = quinn::TransportConfig::default();
        transport_config.max_idle_timeout(None);
        client_config.transport_config(Arc::new(transport_config));

        let mut endpoint = quinn::Endpoint::client("[::]:0".parse().unwrap())?;
        endpoint.set_default_client_config(client_config);

        let connection = endpoint.connect(socket_addr, &hostname)?.await?;
        let (sender, receiver) = connection.open_bi().await.map_err(anyhow::Error::from)?;
        let stream = QuicStream { sender, receiver };

        Ok(QuicRawConnection {
            conn: connection,
            stream,
        })
    }

    fn establish(
        &self,
        raw_conn: Self::RawConnection,
        _is_server: bool,
    ) -> anyhow::Result<Self::Connection> {
        Ok(QuicConnection(raw_conn.conn))
    }

    async fn abolish(&self, mut raw_conn: Self::RawConnection) {
        let _ = raw_conn.stream.sender.finish();
        let _ = raw_conn.stream.sender.stopped().await;
    }
}

#[async_trait]
impl LogicConnection for QuicConnection {
    type Stream = QuicStream;

    async fn accept(&self) -> anyhow::Result<Self::Stream> {
        let (sender, receiver) = self.0.accept_bi().await?;
        Ok(QuicStream { sender, receiver })
    }

    async fn open(&self) -> anyhow::Result<Self::Stream> {
        let (sender, receiver) = self.0.open_bi().await?;
        Ok(QuicStream { sender, receiver })
    }
}

impl AsyncRead for QuicRawConnection {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().stream.receiver).poll_read(cx, buf)
    }
}

impl AsyncWrite for QuicRawConnection {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.get_mut().stream.sender)
            .poll_write(cx, buf)
            .map_err(std::io::Error::from)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.get_mut().stream.sender).poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.get_mut().stream.sender).poll_shutdown(cx)
    }
}

impl AsyncRead for QuicStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().receiver).poll_read(cx, buf)
    }
}

impl AsyncWrite for QuicStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.get_mut().sender)
            .poll_write(cx, buf)
            .map_err(std::io::Error::from)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.get_mut().sender).poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.get_mut().sender).poll_shutdown(cx)
    }
}
