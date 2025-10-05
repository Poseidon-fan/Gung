use std::{
    net::{SocketAddr, ToSocketAddrs},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use async_trait::async_trait;
use quinn::{
    Endpoint,
    crypto::rustls::{QuicClientConfig, QuicServerConfig},
};
use rustls::RootCertStore;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::{
    LogicConnection, Transport,
    option::{TlsServerOption, TransportClientOption, TransportServerOption},
};
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
    stream: QuicStream,
}

#[async_trait]
impl Transport for QuicTransport {
    type Listener = quinn::Endpoint;
    type RawConnection = QuicRawConnection;
    type Connection = QuicConnection;
    type Channel = QuicStream;

    async fn bind<T: ToSocketAddrs + Send>(
        &self,
        addr: T,
        option: TransportServerOption,
    ) -> anyhow::Result<Self::Listener> {
        let TransportServerOption::Quic(quic_option) = option else {
            return Err(anyhow!("Expected Quic transport option"));
        };
        let TlsServerOption { cert, key } = quic_option.tls;

        let mut server_crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert, key)?;
        server_crypto.alpn_protocols = ALPN_GUNG.iter().map(|&x| x.into()).collect();
        let mut server_config =
            quinn::ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(server_crypto)?));
        let transport_config = Arc::get_mut(&mut server_config.transport).unwrap();
        transport_config.max_concurrent_uni_streams(0_u8.into());

        Ok(Endpoint::server(
            server_config,
            addr.to_socket_addrs()?.next().unwrap(),
        )?)
    }

    async fn accept(
        &self,
        l: &Self::Listener,
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
        option: TransportClientOption,
    ) -> anyhow::Result<Self::RawConnection> {
        let socket_addr = addr.to_socket_addrs()?.next().unwrap();
        let default_hostname = socket_addr.ip().to_string();
        let TransportClientOption::Quic(quic_option) = option else {
            return Err(anyhow!("Expected Quic transport option"));
        };
        let hostname = quic_option
            .tls
            .hostname
            .as_ref()
            .unwrap_or(&default_hostname);

        let mut roots = RootCertStore::empty();
        for cert in rustls_native_certs::load_native_certs().expect("could not load platform certs")
        {
            roots.add(cert).unwrap();
        }
        if let Some(cert) = quic_option.tls.cert {
            for cert in cert {
                roots.add(cert).unwrap();
            }
        }
        let mut client_crypto = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        client_crypto.alpn_protocols = ALPN_GUNG.iter().map(|&x| x.into()).collect();
        let client_config =
            quinn::ClientConfig::new(Arc::new(QuicClientConfig::try_from(client_crypto)?));
        let mut endpoint = quinn::Endpoint::client("[::]:0".parse().unwrap())?;
        endpoint.set_default_client_config(client_config);

        let connection = endpoint.connect(socket_addr, hostname)?.await?;
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
