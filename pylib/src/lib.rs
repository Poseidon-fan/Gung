use pyo3::prelude::*;

#[pymodule]
fn gung(m: &Bound<'_, PyModule>) -> PyResult<()> {
    auth::register_module(m)?;
    Ok(())
}
