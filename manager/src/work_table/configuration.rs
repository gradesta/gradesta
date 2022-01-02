use std::path::Path;
use std::time::Duration;

pub struct Configuration {
    sockets_dir: Box<Path>,
    init: Box<Path>,
    service_heartbeat_timeout: Duration,
}
