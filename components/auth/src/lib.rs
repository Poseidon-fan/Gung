mod impls;
mod msg;

use anyhow::Result;
use async_trait::async_trait;
use pyo3::prelude::*;

use crate::msg::*;

pub use impls::*;

#[async_trait]
pub trait Authenticate {
    async fn authenticate(&self, ctx: AuthContext) -> Result<AuthResp>;
}

pub fn register_module(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let auth_module = PyModule::new(parent_module.py(), "auth")?;
    auth_module.add_class::<AuthReq>()?;
    auth_module.add_class::<AuthResp>()?;
    auth_module.add_class::<AuthContext>()?;
    auth_module.add_class::<AuthType>()?;
    auth_module.add_class::<AuthAcceptResp>()?;
    auth_module.add_class::<AuthRejectResp>()?;
    auth_module.add_class::<AuthChallengeResp>()?;
    parent_module.add_submodule(&auth_module)?;
    Ok(())
}
