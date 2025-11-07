use std::net::SocketAddr;

use anyhow::Result;
use async_trait::async_trait;
use hyper::{Request, Response, body::Incoming, service::service_fn};
use hyper_util::rt::TokioIo;
use multi_index_map::MultiIndexMap;
use parking_lot::RwLock;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
    sync::oneshot,
};

use crate::proxy::{Gateway, Proxy, ProxyHandle};

pub struct HttpProxy {}

#[derive(MultiIndexMap)]
struct HttpRouter {
    #[multi_index(hashed_unique)]
    proxy_id: String,
    #[multi_index(hashed_unique)]
    host: String,
    handle: ProxyHandle<HttpProxy>,
}

pub struct HttpGateway {
    listener: TcpListener,
    router: RwLock<MultiIndexHttpRouterMap>,
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
                router: RwLock::new(MultiIndexHttpRouterMap::default()),
            })
    }

    async fn accept(&self) -> Result<(Self::RawStream, SocketAddr)> {
        self.listener.accept().await.map_err(anyhow::Error::from)
    }

    async fn dispatch(&self, stream: Self::RawStream) {
        let service = service_fn({
            let router = &self.router;
            move |req: Request<Incoming>| {
                let host = req
                    .headers()
                    .get("host")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();
                println!("host: {}", host);
                let req_tx = router
                    .read()
                    .get_by_host("127.0.0.1")
                    .map(|router| router.handle.req_tx.clone());
                async move {
                    if let Some(req_tx) = req_tx {
                        let (resp_tx, resp_rx) = oneshot::channel();
                        let http_req = HttpRequest { req, resp_tx };
                        let _ = req_tx.send(http_req);
                        resp_rx.await.unwrap()
                    } else {
                        todo!()
                    }
                }
            }
        });
        let io_stream = TokioIo::new(stream);
        let _ = hyper::server::conn::http1::Builder::new()
            .serve_connection(io_stream, service)
            .await;
    }

    fn add_proxy(&self, handle: ProxyHandle<Self::Proxy>, _config: &config::client::ProxyConfig) {
        self.router.write().insert(HttpRouter {
            proxy_id: handle.proxy_id.clone(),
            host: "127.0.0.1".to_string(),
            handle,
        });
    }

    fn remove_proxy(&self, proxy_id: String) {
        self.router.write().remove_by_proxy_id(&proxy_id);
    }

    fn is_empty(&self) -> bool {
        self.router.read().is_empty()
    }
}
