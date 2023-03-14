extern crate notify;

use notify::{Watcher};
use tokio::sync::mpsc::channel;
use tokio::sync::mpsc;
use std::path::Path;
use anyhow;

#[derive(Debug, PartialEq)]
enum UnixSocketEvent {
    NewSocket(String),
    SocketClosed(String),
}

struct SocketDirWatcher {
    sender: mpsc::Sender<UnixSocketEvent>,
    receiver: mpsc::Receiver<notify::Result<notify::event::Event>>,
}

impl SocketDirWatcher {
    fn new(sender: mpsc::Sender<UnixSocketEvent>, path: String) -> anyhow::Result<Self> {
        let (tx_socket_dir_changes, rx_socket_dir_changes) = channel(100);
        let mut watcher: notify::RecommendedWatcher =
            Watcher::new(tx_socket_dir_changes, notify::Config::default())?;
        watcher.watch(Path::new(&path), notify::RecursiveMode::Recursive)?;

        Ok(SocketDirWatcher {
            sender,
            receiver: rx_socket_dir_changes,
        })
    }
    fn process_event(&mut self, event: notify::event::Event) -> anyhow::Result<()> {
        match event.kind {
            notify::EventKind::Create(notify::event::CreateKind::Folder) => {
                for(_, path) in event.paths.iter().enumerate() {
                    let socket_url: String = format!("ipc://{}/PAIR.zmq", path.to_str().unwrap());
                    self.sender.send(UnixSocketEvent::NewSocket(socket_url)).await?;
                }
                Ok(())
            },
            _ => {
                Ok(())
            }
        }
    }
}

/// Watches for new sockets in the socket directory.
/// Sends evets to the Sender in the form of a UnixSocketEvent.
///
/// Run this with tokio::spawn..
/// Example:
/// ```
///  let (sender, receiver) = mpsc::channel(8);
///  let watcher = SocketDirWatcher::new(sender, Path::new("/tmp/sockets_dir"));
///  tokio::spawn(watch_for_new_sockets(watcher));
/// ```
async fn watch_for_new_sockets(mut watcher: SocketDirWatcher) {
    loop {
        match watcher.receiver.recv() {
            Ok(Ok(event)) => watcher.process_event(event),
            Ok(Err(e)) => {
                println!("watch error: {:?}", e);
                Ok(())
            },
            Err(e) => {
                assert!(false, "{:?}", e);
                // Not sure why but I'm getting this error:
                // thread 'ageing_cellar::watch_sockets_dir::tests::test_socket_dir_watcher' panicked at 'RecvError', src/ageing_cellar/watch_sockets_dir.rs:67:17
                // note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
                break;
            }
        }.unwrap();
    }
    assert!(false, "watch_for_new_sockets should never exit");
}


// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_socket_dir_watcher() {
        let (sender, receiver) = mpsc::channel(8);
        // Delete previous test socket dir if it exists
        let _ = tokio::fs::remove_dir_all("/tmp/sockets_dir");
        tokio::fs::create_dir_all("/tmp/sockets_dir").await.unwrap();
        let watcher = SocketDirWatcher::new(sender, "/tmp/sockets_dir".to_string()).unwrap();
        tokio::spawn(watch_for_new_sockets(watcher));
        // Create new socket directory
        tokio::fs::create_dir_all("/tmp/sockets_dir/new-socket-dir").await.unwrap();
        let event = receiver.recv().await.unwrap();


        assert_eq!(event, UnixSocketEvent::NewSocket("ipc:///tmp/sockets_dir/new-socket-dir/PAIR.zmq".to_string()));
    }
}
