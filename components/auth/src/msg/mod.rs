mod codec;

use std::net::{IpAddr, SocketAddr};

use pyo3::{exceptions::PyValueError, prelude::*};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

pub use codec::*;

#[pyclass]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthReq {
    pub payload: JsonValue,
}

#[pyclass]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthResp {
    Accept(AuthAcceptResp),
    Reject(AuthRejectResp),
    Challenge(AuthChallengeResp),
}

#[pyclass]
#[derive(Clone)]
pub struct AuthContext {
    pub auth_id: String,
    pub server_ip: IpAddr,

    pub client_version: Version,
    pub client_addr: SocketAddr,
    #[pyo3(get)]
    pub proxy: config::client::ProxyConfig,
    #[pyo3(get)]
    pub requests: Vec<AuthReq>,
    #[pyo3(get)]
    pub responses: Vec<AuthResp>,
}

#[pyclass]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthAcceptResp {
    pub msg: String,
}

#[pyclass]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthRejectResp {
    pub msg: String,
}

#[pyclass]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthChallengeResp {
    #[pyo3(get)]
    pub msg: String,
    #[pyo3(get)]
    pub required_fields: Vec<String>,
}

pub const VERSION_AUTH_FIELD: &str = "__version";
pub const PROXY_AUTH_FIELD: &str = "__proxy";
pub const SERVER_IP_AUTH_FIELD: &str = "__server_ip";

#[pymethods]
impl AuthReq {
    #[getter]
    fn payload(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        pythonize::pythonize(py, &self.payload)
            .map(|bound| bound.unbind())
            .map_err(|e| PyErr::new::<PyValueError, _>(e.to_string()))
    }
}

#[pymethods]
impl AuthResp {
    #[staticmethod]
    pub fn accept(msg: String) -> Self {
        Self::Accept(AuthAcceptResp { msg })
    }
    #[staticmethod]
    pub fn reject(msg: String) -> Self {
        Self::Reject(AuthRejectResp { msg })
    }
    #[staticmethod]
    pub fn challenge(msg: String, required_fields: Vec<String>) -> Self {
        Self::Challenge(AuthChallengeResp {
            msg,
            required_fields,
        })
    }
}

#[pymethods]
impl AuthContext {
    #[getter]
    fn client_version(&self) -> String {
        self.client_version.to_string()
    }
    #[getter]
    fn client_addr(&self) -> String {
        self.client_addr.to_string()
    }

    #[getter]
    pub fn round(&self) -> usize {
        self.requests.len()
    }
}

impl AuthContext {
    pub fn new(
        client_version: Version,
        server_ip: IpAddr,
        client_addr: SocketAddr,
        req: AuthReq,
        proxy: config::client::ProxyConfig,
    ) -> Self {
        Self {
            auth_id: Uuid::new_v4().to_string(),
            requests: vec![req],
            responses: vec![],
            client_version,
            server_ip,
            client_addr,
            proxy,
        }
    }
}
