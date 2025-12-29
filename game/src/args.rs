use bevy::ecs::resource::Resource;
use clap::Parser;

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum Transport {
    Auto,
    Unix,
    Tcp,
}

#[derive(Parser, Resource, Debug, Clone)]
pub struct Args {
    #[clap(short, long)]
    pub synctest: bool,
    #[clap(short, long, default_value = "")]
    pub iroh: String,
    /// Show the person overlay (debug use).
    #[clap(long)]
    pub show_person: bool,
    /// Transport layer: auto uses Unix socket on Unix-like OS, otherwise TCP
    #[clap(long, value_enum, default_value_t = Transport::Auto)]
    pub transport: Transport,
    /// Unix domain socket path (fallback decided per-OS when not provided)
    #[clap(long)]
    pub unix_path: Option<String>,
    /// TCP bind/receive address
    #[clap(long, default_value = "127.0.0.1:45233")]
    pub tcp_addr: String,
}
