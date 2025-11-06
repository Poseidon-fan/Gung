use std::{path::PathBuf, sync::OnceLock};

use anyhow::{Result, anyhow};
use config::server::PythonPluginConfig;
use pyo3::{prelude::*, types::PyList};

pub static PYTHON_PLUGIN_MANAGER: OnceLock<PythonPluginManager> = OnceLock::new();

pub fn init(config: &PythonPluginConfig) -> Result<()> {
    use gung::gung;
    pyo3::append_to_inittab!(gung);
    Python::attach(|py| -> PyResult<()> {
        let syspath = py.import("sys")?.getattr("path")?.cast_into::<PyList>()?;
        syspath.insert(0, config.base_path.clone().to_str())?;
        println!("syspath: {:?}", syspath);
        Ok(())
    })?;

    PYTHON_PLUGIN_MANAGER
        .set(PythonPluginManager {
            base_path: config.base_path.clone(),
        })
        .map_err(|_| anyhow!("Failed to set Python plugin manager"))
}

pub struct PythonPluginManager {
    pub base_path: PathBuf,
}
