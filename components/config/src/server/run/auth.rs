use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AuthConfig {
    #[serde(flatten)]
    pub authenticator: AuthenticatorConfig,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum AuthenticatorConfig {
    #[serde(rename = "pass")]
    Pass,
}
