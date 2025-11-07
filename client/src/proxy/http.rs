use std::{marker::PhantomData, net::SocketAddr};

use anyhow::Result;
use async_trait::async_trait;
use hyper::{Request, body::Incoming, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
use transport::Transport;

use crate::proxy::Proxy;

pub struct HttpProxy<T: Transport> {
    _phantom: PhantomData<T>,
}

#[async_trait]
impl<T: Transport> Proxy for HttpProxy<T> {
    type Stream = T::Channel;

    fn from_config(_config: &config::client::ProxyConfig) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            _phantom: PhantomData,
        })
    }
    async fn handle(&self, stream: Self::Stream, local_addr: SocketAddr) -> Result<()> {
        let service = service_fn(move |req: Request<Incoming>| {
            let local_addr = local_addr;
            async move {
                let local_socket = TcpStream::connect(local_addr)
                    .await
                    .map_err(anyhow::Error::from)?;
                let io_stream = TokioIo::new(local_socket);
                let (mut sender, conn) = hyper::client::conn::http1::Builder::new()
                    .handshake(io_stream)
                    .await?;
                tokio::spawn(conn);
                sender.send_request(req).await.map_err(anyhow::Error::from)
            }
        });

        let io_stream = TokioIo::new(stream);
        hyper::server::conn::http1::Builder::new()
            .serve_connection(io_stream, service)
            .await
            .map_err(anyhow::Error::from)
    }
}
