use pyo3::prelude::*;

#[pymodule]
pub fn gung(m: &Bound<'_, PyModule>) -> PyResult<()> {
    auth::register_module(m)?;
    Ok(())
}
