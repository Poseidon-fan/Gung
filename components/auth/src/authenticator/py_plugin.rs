use anyhow::Result;
use async_trait::async_trait;
use pyo3::prelude::*;

use crate::{AuthContext, AuthResp, Authenticator};

// The authenticator that uses a Python plugin to authenticate the client.
// It will call the `authenticate` function in the Python file.
pub struct PyPluginAuthenticator {
    pub auth_func: Py<PyAny>,
}

#[async_trait]
impl Authenticator for PyPluginAuthenticator {
    async fn authenticate(&self, ctx: &AuthContext) -> Result<AuthResp> {
        Python::attach(|py| {
            let resp = self
                .auth_func
                .call1(py, (ctx.clone(),))?
                .extract::<AuthResp>(py)?;
            Ok(resp)
        })
    }
}

impl PyPluginAuthenticator {
    pub fn new(config: &config::server::PyPluginAuthenticatorConfig) -> Result<Self> {
        Python::attach(|py| {
            let module = py.import(config.file_path.file_stem().unwrap())?;
            let auth_func = module.getattr("authenticate")?;
            Ok(Self {
                auth_func: Py::from(auth_func),
            })
        })
    }
}
