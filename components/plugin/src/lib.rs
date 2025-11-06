use anyhow::Result;
use config::server::PluginConfig;

pub mod python;

pub fn init(config: &PluginConfig) -> Result<()> {
    if let Some(python_config) = &config.python {
        python::init(python_config)?;
    }
    Ok(())
}
