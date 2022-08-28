mod ageing_cellar;
mod work_table;

extern crate clap;
use ageing_cellar::lock_sockets_dir::*;
use work_table::parse_args_and_environment::*;

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

async fn real_main() -> anyhow::Result<()> {
    let config = parse_args_and_environment()?;
    println!("Launching gradesta manager.");
    match &config.sockets_dir {
        Some(sockets_dir) => {
            lock_sockets_dir(sockets_dir)?;
            work_table::clean_socket_dir::clean(sockets_dir)?;
            println!(
                "Will watch for clients binding sockets in {:?}",
                sockets_dir
            );
        }
        None => println!("Unix sockets dissabled."),
    }
    match &config.port {
        Some(port) => println!(
            "Will listen for clients connecting via websockets to port {}",
            port
        ),
        None => println!("Websockets dissabled."),
    }
    let main_loop = work_table::main_loop::run(&config);
    match &config.init {
        Some(init) => {
            let mut builder = std::process::Command::new(init);
            let mut builder = match &config.sockets_dir {
                Some(dir) => builder.arg(format!("--sockets-dir={}", dir.to_string_lossy())),
                None => &mut builder,
            };
            let builder = match &config.port {
                Some(port) => builder.arg(format!("--websockets-port={}", port)),
                None => &mut builder,
            };
            builder.spawn().expect("Error running init binary");
        }
        None => (),
    };
    main_loop.await
}

#[tokio::main]
async fn main() {
    match real_main().await {
        Err(e) => {
            print!("{}", e);
            std::process::exit(1)
        }
        Ok(_) => std::process::exit(0),
    }
}
