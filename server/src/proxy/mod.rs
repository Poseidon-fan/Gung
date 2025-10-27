mod tcp;

use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite},
    select,
    sync::{mpsc, oneshot},
};
use transport::LogicConnection;

#[async_trait]
pub trait Proxy: 'static {
    type Request: Send;

    async fn handle_one<T>(&self, req: Self::Request, channel: T)
    where
        T: AsyncRead + AsyncWrite + Send + Unpin;

    async fn run<T: LogicConnection>(
        self: Arc<Self>,
        proxy_id: String,
        mut req_rx: mpsc::UnboundedReceiver<Self::Request>,
        conn: T,
        external_shutdown_tx: mpsc::Sender<String>,
        mut internal_shutdown_rx: oneshot::Receiver<()>,
    ) -> Result<()> {
        // Request for a control channel
        let mut ctl_channel = conn.open().await?;

        loop {
            select! {
                Some(req) = req_rx.recv() => {
                    let this = self.clone();
                    let data_channel = conn.open().await?;
                    tokio::spawn(async move {
                        this.handle_one(req, data_channel).await;
                    });
                },
                _ = ctl_channel.read_f32() => {
                    // TODO(Poseidon): protocol system for control channel
                    let _ = external_shutdown_tx.send(proxy_id.clone()).await;
                    return Ok(());
                },
                _ = &mut internal_shutdown_rx => {
                    return Ok(());
                }
            }
        }
    }
}

pub struct ProxyHandle<T: Proxy> {
    proxy_id: String,
    req_tx: mpsc::UnboundedSender<T::Request>,
    internal_shutdown_tx: oneshot::Sender<()>,
}

#[async_trait]
pub trait Gateway: 'static {
    type RawStream: Send;
    type Proxy: Proxy;

    async fn accept(&self) -> Result<(Self::RawStream, SocketAddr)>;

    async fn upgrade(raw_stream: Self::RawStream) -> Result<<Self::Proxy as Proxy>::Request>;

    async fn dispatch(&self, req: <Self::Proxy as Proxy>::Request);

    async fn run(self: Arc<Self>) {
        while let Ok((raw_stream, _)) = self.accept().await {
            let this = self.clone();
            tokio::spawn(async move {
                let req = Self::upgrade(raw_stream).await.unwrap();
                this.dispatch(req).await;
            });
        }
    }
}
