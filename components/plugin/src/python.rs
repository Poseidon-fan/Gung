use std::{path::PathBuf, sync::OnceLock};

use anyhow::{Result, anyhow};
use config::server::PythonPluginConfig;
use pyo3::{prelude::*, types::PyList};

pub static PYTHON_PLUGIN_MANAGER: OnceLock<PythonPluginManager> = OnceLock::new();

pub fn init(config: &PythonPluginConfig) -> Result<()> {
    let pkg_path = config.base_pkg_path.clone();
    if !pkg_path.exists() {
        return Err(anyhow!(
            "Python plugin package path does not exist: {}",
            pkg_path.display()
        ));
    }
    if !pkg_path.is_dir() {
        return Err(anyhow!(
            "Python plugin package path is not a directory: {}",
            pkg_path.display()
        ));
    }

    Python::attach(|py| -> PyResult<()> {
        let syspath = py.import("sys")?.getattr("path")?.cast_into::<PyList>()?;
        syspath.insert(0, pkg_path.to_str())?;
        Ok(())
    })?;

    PYTHON_PLUGIN_MANAGER
        .set(PythonPluginManager {
            base_pkg_path: config.base_pkg_path.clone(),
        })
        .map_err(|_| anyhow!("Failed to set Python plugin manager"))
}

pub struct PythonPluginManager {
    pub base_pkg_path: PathBuf,
}
