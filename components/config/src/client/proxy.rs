use clap::{Args, ValueEnum, arg};

#[derive(Debug, Args)]
pub struct ProxyConfig {
    #[arg(long = "proxy", short = 'p')]
    pub proxy_type: ProxyType,
    #[clap(flatten)]
    pub proxy_params: ProxyParams,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum ProxyType {
    Tcp,
    Http,
}

#[derive(Debug, Args)]
pub struct ProxyParams {
    #[arg(long, short = 'r')]
    pub remote_port: Option<u16>,
}
