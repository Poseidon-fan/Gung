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
    #[pyo3(get)]
    pub auth_type: AuthType,
    #[pyo3(get)]
    pub requests: Vec<AuthReq>,
    #[pyo3(get)]
    pub responses: Vec<AuthResp>,
    client_version: Version,
}

#[pyclass]
#[derive(Clone)]
#[repr(u8)]
pub enum AuthType {
    Ping,
    Connect,
}

#[pyclass]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthAcceptResp {
    msg: String,
}

#[pyclass]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthRejectResp {
    msg: String,
}

#[pyclass]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthChallengeResp {
    #[pyo3(get)]
    msg: String,
    #[pyo3(get)]
    required_fields: Vec<String>,
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
    pub fn new(auth_type: AuthType, client_version: Version, req: AuthReq) -> Self {
        Self {
            auth_id: Uuid::new_v4().to_string(),
            auth_type,
            requests: vec![req],
            responses: vec![],
            client_version,
        }
    }
}

impl TryFrom<u8> for AuthType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AuthType::Ping),
            1 => Ok(AuthType::Connect),
            _ => Err(anyhow::anyhow!("invalid auth_type: {}", value)),
        }
    }
}
