use std::net::SocketAddr;

use anyhow::{Result, anyhow};
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
use url::Url;

use crate::proxy::{Gateway, Proxy, ProxyHandle};

pub struct HttpProxy {}

#[derive(MultiIndexMap)]
struct HttpRouter {
    #[multi_index(hashed_unique)]
    proxy_id: String,
    #[multi_index(hashed_unique)]
    domain: String,
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

    async fn handle_one<T>(&self, req: Self::Request, channel: T) -> Result<()>
    where
        T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        let io_stream = TokioIo::new(channel);
        // let _host = req.req.uri().host().unwrap();
        let (mut sender, conn) = hyper::client::conn::http1::Builder::new()
            .handshake(io_stream)
            .await?;
        tokio::spawn(conn);
        let resp = sender
            .send_request(req.req)
            .await
            .map_err(anyhow::Error::from);
        req.resp_tx
            .send(resp)
            .map_err(|_| anyhow!("Failed to send response"))
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

    async fn dispatch(&self, stream: Self::RawStream) -> Result<()> {
        let service = service_fn({
            let router = &self.router;
            async move |req: Request<Incoming>| {
                let host = req
                    .headers()
                    .get("host")
                    .ok_or(anyhow!("Host not found"))?
                    .to_str()
                    .map(|s| {
                        Url::parse(format!("http://{s}").as_str())
                            .map(|url| url.host_str().unwrap().to_string())
                    })??;

                let req_tx = router
                    .read()
                    .get_by_domain(&host)
                    .map(|router| router.handle.req_tx.clone())
                    .ok_or(anyhow!("Proxy not found"))?;

                let (resp_tx, resp_rx) = oneshot::channel();
                let http_req = HttpRequest { req, resp_tx };
                req_tx.send(http_req)?;
                resp_rx.await?
            }
        });
        let io_stream = TokioIo::new(stream);
        hyper::server::conn::http1::Builder::new()
            .serve_connection(io_stream, service)
            .await
            .map_err(anyhow::Error::from)
    }

    fn add_proxy(
        &self,
        handle: ProxyHandle<Self::Proxy>,
        port: u16,
        config: &config::client::ProxyConfig,
    ) -> Result<String> {
        let domain = match (
            &config.proxy_params.custom_domain,
            &config.proxy_params.sub_domain,
        ) {
            (Some(custom_domain), None) => custom_domain.clone(),
            (None, Some(_sub_domain)) => todo!(),
            (None, None) => todo!(),
            _ => unreachable!(),
        };
        self.router.write().insert(HttpRouter {
            proxy_id: handle.proxy_id.clone(),
            domain: domain.clone(),
            handle,
        });
        Ok(format!("http://{domain}:{port}"))
    }

    fn remove_proxy(&self, proxy_id: String) {
        self.router.write().remove_by_proxy_id(&proxy_id);
    }

    fn is_empty(&self) -> bool {
        self.router.read().is_empty()
    }
}
