use std::path::PathBuf;
use humantime;

pub struct Configuration {
    // Dir for ZMQ unix sockets
    pub sockets_dir: PathBuf,
    // Port for websockets
    pub port: u32,
    // Init binary for launching services after manager startup
    pub init: Option<PathBuf>,
    // Amount of time to wait after heartbeat before closing connection
    pub service_heartbeat_timeout: humantime::Duration,
}
