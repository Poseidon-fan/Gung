use clap::{Args, ValueEnum};
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[pyclass]
#[derive(Debug, Args, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    #[pyo3(get)]
    #[arg(long = "proxy", short = 'p')]
    pub proxy_type: ProxyType,
    #[pyo3(get)]
    #[clap(flatten)]
    pub proxy_params: ProxyParams,
}

#[pyclass]
#[derive(Debug, Clone, ValueEnum, EnumString, Display, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
pub enum ProxyType {
    Tcp,
    Http,
}

#[pyclass]
#[derive(Debug, Args, Clone, Serialize, Deserialize)]
pub struct ProxyParams {
    #[pyo3(get)]
    #[arg(long, short = 'r')]
    pub remote_port: Option<u16>,
    #[pyo3(get)]
    #[arg(long, conflicts_with = "sub_domain")]
    pub custom_domain: Option<String>,
    #[pyo3(get)]
    #[arg(long, conflicts_with = "custom_domain")]
    pub sub_domain: Option<String>,
}
