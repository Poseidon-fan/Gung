mod msg;

use pyo3::prelude::*;

use crate::msg::AuthReq;

pub fn register_module(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let auth_module = PyModule::new(parent_module.py(), "auth")?;
    auth_module.add_class::<AuthReq>()?;
    parent_module.add_submodule(&auth_module)?;
    Ok(())
}
