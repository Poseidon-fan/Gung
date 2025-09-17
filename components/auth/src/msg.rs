use pyo3::{exceptions::PyValueError, prelude::*};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[pyclass]
#[derive(Serialize, Deserialize)]
pub struct AuthReq {
    payload: Value,
}

#[pymethods]
impl AuthReq {
    #[new]
    fn new(payload: Py<PyAny>, py: Python) -> PyResult<Self> {
        let value = pythonize::depythonize(payload.bind(py))?;
        Ok(Self { payload: value })
    }

    #[getter]
    fn payload(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        pythonize::pythonize(py, &self.payload)
            .map(|bound| bound.unbind())
            .map_err(|e| PyErr::new::<PyValueError, _>(e.to_string()))
    }
}
