use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PluginConfig {
    pub python: Option<PythonPluginConfig>,
}

#[derive(Debug, Deserialize)]
pub struct PythonPluginConfig {
    pub base_pkg_path: PathBuf,
}
