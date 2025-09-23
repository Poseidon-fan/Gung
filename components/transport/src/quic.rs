#![allow(dead_code, unused_variables)]
use anyhow::Result;
use async_trait::async_trait;
use quinn::{Endpoint, Incoming, RecvStream, SendStream, rustls::Connection};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{ToSocketAddrs, unix::SocketAddr},
};

use crate::Transport;

pub struct QuicTransport {}

pub struct QuicListener(Endpoint, Incoming);

pub struct QuicStream {
    connection: Connection,
    rx: RecvStream,
    tx: SendStream,
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

    fn poll_write_vectored(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let buf = bufs
            .iter()
            .find(|b| !b.is_empty())
            .map_or(&[][..], |b| &**b);
        self.poll_write(cx, buf)
    }

    fn is_write_vectored(&self) -> bool {
        false
    }
}

impl Drop for QuicStream {
    fn drop(&mut self) {
        todo!()
    }
}

#[async_trait]
impl Transport for QuicTransport {
    type Listener = QuicListener;
    type Stream = QuicStream;

    async fn bind<T: ToSocketAddrs + Send + Sync>(&self, addr: T) -> Result<Self::Listener> {
        todo!()
    }

    async fn accept(&self, l: &mut Self::Listener) -> Result<(Self::Stream, SocketAddr)> {
        todo!()
    }

    async fn connect<T: ToSocketAddrs + Send + Sync>(&self, addr: T) -> Result<Self::Stream> {
        todo!()
    }

    async fn close(&self, l: Self::Listener) {
        todo!()
    }
}
