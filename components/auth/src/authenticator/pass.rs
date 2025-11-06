use anyhow::Result;
use async_trait::async_trait;

use crate::{
    Authenticator,
    msg::{AuthContext, AuthResp},
};

#[derive(Default)]
pub struct PassAuthenticator();

impl PassAuthenticator {
    pub fn new() -> Self {
        Self()
    }
}

#[async_trait]
impl Authenticator for PassAuthenticator {
    async fn authenticate(&self, _ctx: &AuthContext) -> Result<AuthResp> {
        Ok(AuthResp::accept("pass".to_string()))
    }
}
