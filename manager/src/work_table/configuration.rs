use std::path::PathBuf;
use std::env::var;
use humantime;
use clap::Parser;

static default_sockets_dir: &str = &format!("{}/.cache/gradsta/services/", std::env::var("HOME").unwrap());

/// Manage and route messages between gradesta protocol services. Provides the meta namespace. Evaluates walk trees.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Configuration {
    /// Directory where peer sockets are connected
    #[clap(short, long, default_value = default_sockets_dir)]
    pub sockets_dir: PathBuf,
    /// Path to the init executable to be executed after the manager has launched
    #[clap(short, long)]
    pub init: PathBuf,
    /// How long should the manager wait for a heartbeat response before closing the connection to a peer or client and cleaning up.
    #[clap(short='h', long, default_value_t = std::time::Duration::new(15, 0).into())]
    pub service_heartbeat_timeout: humantime::Duration,
}
