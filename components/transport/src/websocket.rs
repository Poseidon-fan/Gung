use std::{
    io::{Error, ErrorKind},
    net::{SocketAddr, ToSocketAddrs},
    pin::Pin,
    task::{Context, Poll, ready},
};

use anyhow::{Context as AnyhowContext, anyhow};
use async_trait::async_trait;
use bytes::Bytes;
use config::server::ProtocolConfig;
use futures_util::{Sink, stream::Stream};
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf},
    net::{TcpListener, TcpStream},
};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{Message, protocol::WebSocketConfig},
};
use tokio_util::io::StreamReader;

use crate::{MaybeTlsStream, Transport, load_client_tls_acceptor, load_server_tls_acceptor};

pub struct WebsocketTransport {
    websocket_config: WebSocketConfig,
    tls_acceptor: Option<TlsAcceptor>,
    tls_connector: Option<TlsConnector>,
}

pub struct WebsocketTransportClientOption {
    hostname: Option<String>,
}

pub struct WebsocketTransportServerOption {}

// A wrapper that converts the message protocol of websocket to bytes interface.
pub struct WsBytesAdapter(WebSocketStream<TcpStream>);

// Furthermore, wrap bytes stream to`AsyncRead` and `AsyncWrite` traits.
pub struct WsStream(StreamReader<WsBytesAdapter, Bytes>);

#[async_trait]
impl Transport for WebsocketTransport {
    type Listener = TcpListener;
    type RawConnection = MaybeTlsStream<WsStream>;
    type Connection = net_mux::Session<MaybeTlsStream<WsStream>>;
    type Channel = net_mux::Stream;
    type TransportClientOption = WebsocketTransportClientOption;
    type TransportServerOption = WebsocketTransportServerOption;

    fn new_server(
        config: &config::server::TransportConfig,
    ) -> anyhow::Result<(Self, Self::TransportServerOption)> {
        let ProtocolConfig::Websocket(websocket_transport_config) = &config.protocol else {
            return Err(anyhow!("Invalid protocol config"));
        };
        let mut websocket_config = WebSocketConfig::default();
        websocket_config.write_buffer_size = 0;
        Ok((
            Self {
                websocket_config,
                tls_acceptor: load_server_tls_acceptor(&websocket_transport_config.tls)?,
                tls_connector: None,
            },
            Self::TransportServerOption {},
        ))
    }

    fn new_client(
        config: &config::client::TransportConfig,
    ) -> anyhow::Result<(Self, Self::TransportClientOption)> {
        let mut websocket_config = WebSocketConfig::default();
        websocket_config.write_buffer_size = 0;
        let hostname = config.transport_params.hostname.clone();
        Ok((
            Self {
                websocket_config,
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
        let (tcp_stream, addr) = l.accept().await.map_err(anyhow::Error::from)?;
        let ws_raw_stream =
            tokio_tungstenite::accept_async_with_config(tcp_stream, Some(self.websocket_config))
                .await?;
        let raw_stream = WsStream(StreamReader::new(WsBytesAdapter(ws_raw_stream)));
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
        let tcp_stream = TcpStream::connect(addr).await?;
        let (ws_raw_stream, _) = tokio_tungstenite::client_async_with_config(
            format!("ws://{}", &addr.to_string()),
            tcp_stream,
            Some(self.websocket_config),
        )
        .await?;
        let raw_stream = WsStream(StreamReader::new(WsBytesAdapter(ws_raw_stream)));
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

impl Stream for WsBytesAdapter {
    type Item = Result<Bytes, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.get_mut().0).poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(Error::other(err)))),
            Poll::Ready(Some(Ok(res))) => {
                if let Message::Binary(b) = res {
                    Poll::Ready(Some(Ok(b)))
                } else {
                    Poll::Ready(Some(Err(Error::new(
                        ErrorKind::InvalidData,
                        "unexpected frame",
                    ))))
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl AsyncRead for WsStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().0).poll_read(cx, buf)
    }
}

impl AsyncWrite for WsStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let sw = self.get_mut().0.get_mut();
        ready!(Pin::new(&mut sw.0).poll_ready(cx).map_err(Error::other))?;

        match Pin::new(&mut sw.0).start_send(Message::Binary(buf.to_vec().into())) {
            Ok(()) => Poll::Ready(Ok(buf.len())),
            Err(e) => Poll::Ready(Err(Error::other(e))),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.get_mut().0.get_mut().0)
            .poll_flush(cx)
            .map_err(Error::other)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.get_mut().0.get_mut().0)
            .poll_close(cx)
            .map_err(Error::other)
    }
}
