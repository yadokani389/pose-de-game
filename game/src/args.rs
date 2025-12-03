use bevy::ecs::resource::Resource;
use clap::Parser;

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum Transport {
    Auto,
    Unix,
    Udp,
}

#[derive(Parser, Resource, Debug, Clone)]
pub struct Args {
    #[clap(short, long)]
    pub synctest: bool,
    #[clap(short, long, default_value = "")]
    pub iroh: String,
    /// Transport layer: auto uses Unix socket on Unix-like OS, otherwise UDP
    #[clap(long, value_enum, default_value_t = Transport::Auto)]
    pub transport: Transport,
    /// Unix domain socket path (fallback decided per-OS when not provided)
    #[clap(long)]
    pub unix_path: Option<String>,
    /// UDP bind/receive address
    #[clap(long, default_value = "127.0.0.1:45233")]
    pub udp_addr: String,
}
