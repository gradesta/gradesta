use humantime;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Configuration {
    // Dir for ZMQ unix sockets
    pub sockets_dir: Option<PathBuf>,
    // Port for websockets
    pub port: Option<u32>,
    // Init binary for launching services after manager startup
    pub init: Option<PathBuf>,
    // Amount of time to wait after heartbeat before closing connection
    pub service_heartbeat_timeout: humantime::Duration,
}
