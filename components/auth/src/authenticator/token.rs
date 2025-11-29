use anyhow::{Context, Result};
use async_trait::async_trait;

use crate::{
    Authenticator,
    msg::{AuthContext, AuthResp},
};

// This authenticator checks if the `token` field in the request payload corresponds to the given one.
pub struct TokenAuthenticator(String);

impl TokenAuthenticator {
    pub fn new(token: String) -> Self {
        Self(token)
    }
}

#[async_trait]
impl Authenticator for TokenAuthenticator {
    async fn authenticate(&self, ctx: &AuthContext) -> Result<AuthResp> {
        const MAX_ROUND: usize = 3;
        match ctx.round() {
            m if m > MAX_ROUND => return Ok(AuthResp::reject("round limit exceeded".to_string())),
            _ => ctx
                .requests
                .last()
                .with_context(|| "Failed when getting last response")?
                .payload
                .get("token")
                .map_or_else(
                    || {
                        Ok(AuthResp::challenge(
                            "token is required".to_string(),
                            vec!["token".to_string()],
                        ))
                    },
                    |token| {
                        println!("token string: {}", token);
                        token.as_str().map_or_else(
                            || {
                                Ok(AuthResp::challenge(
                                    "token must be a string".to_string(),
                                    vec!["token".to_string()],
                                ))
                            },
                            |token| {
                                if token == self.0 {
                                    Ok(AuthResp::accept("token is correct".to_string()))
                                } else {
                                    Ok(AuthResp::challenge(
                                        "token is incorrect".to_string(),
                                        vec!["token".to_string()],
                                    ))
                                }
                            },
                        )
                    },
                ),
        }
    }
}
