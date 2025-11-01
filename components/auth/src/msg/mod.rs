#![allow(dead_code)]
mod codec;

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
pub struct AuthContext {
    pub auth_id: String,
    pub client_version: Version,
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
impl AuthAcceptResp {
    #[new]
    pub fn new(msg: String) -> Self {
        Self { msg }
    }
}

#[pymethods]
impl AuthRejectResp {
    #[new]
    fn new(msg: String) -> Self {
        Self { msg }
    }
}

#[pymethods]
impl AuthChallengeResp {
    #[new]
    fn new(msg: String, required_fields: Vec<String>) -> Self {
        Self {
            msg,
            required_fields,
        }
    }
}

#[pymethods]
impl AuthContext {
    #[getter]
    fn client_version(&self) -> String {
        self.client_version.to_string()
    }
}

impl AuthContext {
    pub fn new(client_version: Version, req: AuthReq, proxy: config::client::ProxyConfig) -> Self {
        Self {
            auth_id: Uuid::new_v4().to_string(),
            requests: vec![req],
            responses: vec![],
            client_version,
            proxy,
        }
    }
}
