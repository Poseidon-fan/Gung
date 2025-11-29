use std::path::PathBuf;

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
