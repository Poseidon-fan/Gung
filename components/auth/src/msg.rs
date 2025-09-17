use pyo3::{exceptions::PyValueError, prelude::*};
use serde_json::Value;

#[pyclass]
pub struct AuthReq {
    payload: Value,
}

#[pyclass]
pub enum AuthResp {
    Accept(AuthAcceptResp),
    Reject(AuthRejectResp),
    Challenge(AuthChallengeResp),
}

#[pyclass]
#[derive(Clone)]
pub struct AuthAcceptResp {
    #[pyo3(get)]
    msg: String,
}

#[pyclass]
#[derive(Clone)]
pub struct AuthRejectResp {
    #[pyo3(get)]
    msg: String,
}

#[pyclass]
#[derive(Clone)]
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
    fn new(msg: String) -> Self {
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
