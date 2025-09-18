use anyhow::Result;
use async_trait::async_trait;

use crate::{
    Authenticate,
    msg::{AuthAcceptResp, AuthContext, AuthResp},
};

#[derive(Default)]
pub struct PassAuthenticator();

impl PassAuthenticator {
    pub fn new() -> Self {
        Self()
    }
}

#[async_trait]
impl Authenticate for PassAuthenticator {
    async fn authenticate(&self, _ctx: AuthContext) -> Result<AuthResp> {
        Ok(AuthResp::Accept(AuthAcceptResp::new("pass".to_string())))
    }
}
