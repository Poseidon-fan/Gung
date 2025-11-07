use std::net::SocketAddr;

use anyhow::Result;
use async_trait::async_trait;
use hyper::{Request, Response, body::Incoming, service::service_fn};
use hyper_util::rt::TokioIo;
use parking_lot::Mutex;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
    sync::oneshot,
};

use crate::proxy::{Gateway, Proxy, ProxyHandle};

pub struct HttpProxy {}

pub struct HttpGateway {
    listener: TcpListener,
    proxy_handle: Mutex<Option<ProxyHandle<HttpProxy>>>,
}

pub struct HttpRequest {
    req: Request<Incoming>,
    resp_tx: oneshot::Sender<Result<Response<Incoming>>>,
}

#[async_trait]
impl Proxy for HttpProxy {
    type Request = HttpRequest;

    fn from_client_config(_config: &config::client::ProxyConfig) -> Result<Self> {
        Ok(Self {})
    }

    async fn handle_one<T>(&self, req: Self::Request, channel: T)
    where
        T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        let io_stream = TokioIo::new(channel);
        // let _host = req.req.uri().host().unwrap();
        let (mut sender, conn) = hyper::client::conn::http1::Builder::new()
            .handshake(io_stream)
            .await
            .unwrap();
        tokio::spawn(conn);
        let resp = sender
            .send_request(req.req)
            .await
            .map_err(anyhow::Error::from);
        let _ = req.resp_tx.send(resp);
    }
}

#[async_trait]
impl Gateway for HttpGateway {
    type RawStream = TcpStream;
    type Proxy = HttpProxy;

    async fn bind(addr: SocketAddr) -> Result<Self> {
        TcpListener::bind(addr)
            .await
            .map_err(anyhow::Error::from)
            .map(|listener| Self {
                listener,
                proxy_handle: Mutex::new(None),
            })
    }

    async fn accept(&self) -> Result<(Self::RawStream, SocketAddr)> {
        self.listener.accept().await.map_err(anyhow::Error::from)
    }

    async fn dispatch(&self, stream: Self::RawStream) {
        let io_stream = TokioIo::new(stream);
        let req_tx = self.proxy_handle.lock().as_ref().unwrap().req_tx.clone();
        let service = service_fn(move |req: Request<Incoming>| {
            let req_tx = req_tx.clone();
            async move {
                let (resp_tx, resp_rx) = oneshot::channel();
                let http_req = HttpRequest { req, resp_tx };
                let _ = req_tx.send(http_req);
                resp_rx.await.unwrap()
            }
        });
        let _ = hyper::server::conn::http1::Builder::new()
            .serve_connection(io_stream, service)
            .await;
    }

    fn add_proxy(&self, handle: ProxyHandle<Self::Proxy>, _config: &config::client::ProxyConfig) {
        *self.proxy_handle.lock() = Some(handle);
    }

    fn remove_proxy(&self, proxy_id: String) {
        let handle = self.proxy_handle.lock().take();
        if let Some(handle) = handle {
            debug_assert_eq!(handle.proxy_id, proxy_id);
            let _ = handle.server_shutdown_tx.send(());
        }
    }

    fn is_empty(&self) -> bool {
        self.proxy_handle.lock().is_none()
    }
}
