mod ageing_cellar;
mod work_table;

extern crate clap;
use work_table::parse_args_and_environment::*;
use ageing_cellar::lock_sockets_dir::*;

/*
extern crate notify;

use notify::DebouncedEvent::Create;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use std::time::Duration;

use zmq;


fn watch(path: String) -> notify::Result<()> {
    println!("Watching {}", path);
    // Create a channel to receive the events.
    let (tx_socket_dir_changes, rx_socket_dir_changes) = channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher: RecommendedWatcher =
        Watcher::new(tx_socket_dir_changes, Duration::from_secs(2))?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path, RecursiveMode::Recursive)?;

    let mut sockets: Vec<zmq::Socket> = vec![];
    let ctx = zmq::Context::new();

    // This is a simple loop, but you may want to use more complex logic here,
    // for example to handle I/O.
    loop {
        match rx_socket_dir_changes.recv() {
            Ok(Create(path)) => {
                let mut socket = ctx.socket(zmq::REQ).unwrap();
                let socket_url: String = format!("ipc://{}/PAIR.zmq", path.to_str().unwrap());
                println!("Connecting to {}", socket_url);
                socket.connect(&socket_url).unwrap();
            }
            Err(e) => println!("watch error: {:?}", e),
            Ok(event) => {
                println!("Unexpected filesystem event. {:?}", event);
            }
        }
    }
}
 */

async fn real_main() -> anyhow::Result<()>  {
    let config = parse_args_and_environment()?;
    lock_sockets_dir(&config.sockets_dir)?;
    Ok(())
}

#[tokio::main]
async fn main() {
    match real_main().await {
        Err(e) => {
            print!("{}", e);
            std::process::exit(1)
        },
        Ok(_) => std::process::exit(0)
    }
}
