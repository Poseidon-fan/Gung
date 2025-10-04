#![allow(dead_code, unused_variables)]

use async_trait::async_trait;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::unix::SocketAddr,
};

use crate::{LogicConnection, Transport};

pub struct QuicTransport {}

#[async_trait]
impl Transport for QuicTransport {
    type Listener = quinn::Endpoint;
    type RawConnection = QuicRawConnection;
    type Connection = QuicConnection;
    type Channel = QuicStream;

    async fn bind(addr: SocketAddr) -> anyhow::Result<Self::Listener> {
        todo!()
    }

    async fn accept(l: &mut Self::Listener) -> anyhow::Result<(Self::RawConnection, SocketAddr)> {
        todo!()
    }

    async fn connect(addr: SocketAddr) -> anyhow::Result<Self::RawConnection> {
        todo!()
    }

    fn establish(
        raw_conn: Self::RawConnection,
        is_server: bool,
    ) -> anyhow::Result<Self::Connection> {
        todo!()
    }
}

pub struct QuicConnection(quinn::Connection);

pub struct QuicStream {
    sender: quinn::SendStream,
    receiver: quinn::RecvStream,
}

pub struct QuicRawConnection {
    conn: quinn::Connection,
    stream: QuicStream,
}

impl AsyncRead for QuicRawConnection {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        todo!()
    }
}

impl AsyncWrite for QuicRawConnection {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        todo!()
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        todo!()
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        todo!()
    }
}

#[async_trait]
impl LogicConnection for QuicConnection {
    type Stream = QuicStream;

    async fn accept() -> anyhow::Result<Self::Stream> {
        todo!()
    }

    async fn open() -> anyhow::Result<Self::Stream> {
        todo!()
    }
}

impl AsyncRead for QuicStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        todo!()
    }
}

impl AsyncWrite for QuicStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        todo!()
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        todo!()
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        todo!()
    }
}
