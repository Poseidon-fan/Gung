use std::path::PathBuf;

use crate::default_u64;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AuthConfig {
    #[serde(flatten)]
    pub authenticator: AuthenticatorConfig,
    /// Timeout for a single authentication attempt (in seconds).
    #[serde(default = "default_u64::<30>")]
    pub timeout: u64,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum AuthenticatorConfig {
    #[serde(rename = "pass")]
    Pass,
    #[serde(rename = "py_plugin")]
    PyPlugin(PyPluginAuthenticatorConfig),
    #[serde(rename = "token")]
    Token(TokenAuthenticatorConfig),
}

#[derive(Debug, Deserialize)]
pub struct PyPluginAuthenticatorConfig {
    pub file_path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct TokenAuthenticatorConfig {
    pub token: String,
}
