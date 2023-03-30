extern crate notify;

use notify::{Watcher, RecommendedWatcher};

use super::localizer::*;

use tokio::sync::mpsc::channel;
use tokio::sync::mpsc;
use tokio::runtime::Handle;
use std::path::Path;
use anyhow;

#[derive(Debug, PartialEq)]
enum UnixSocketEvent {
    NewSocket(String),
    SocketClosed(String),
}


struct SocketDirWatcher {
    pub sender: mpsc::Sender<UnixSocketEvent>,
    watcher: RecommendedWatcher,
    receiver: mpsc::Receiver<notify::Result<notify::event::Event>>,
}

impl SocketDirWatcher {
    fn new(sender: mpsc::Sender<UnixSocketEvent>, path: String) -> anyhow::Result<Self> {
        let (tx_socket_dir_changes, rx_socket_dir_changes) = channel(100);
        let handle = Handle::try_current()?;
        let mut watcher = RecommendedWatcher::new(move |res| {
            handle.block_on(async {
                tx_socket_dir_changes.send(res).await.unwrap();
            })
        },
        notify::Config::default())?;
        watcher.watch(Path::new(&path), notify::RecursiveMode::Recursive)?;

        Ok(SocketDirWatcher {
            sender,
            watcher,
            receiver: rx_socket_dir_changes,
        })
    }

    async fn process_event(&mut self, event: notify::event::Event) -> anyhow::Result<()> {
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
async fn watch_for_new_sockets(mut watcher: SocketDirWatcher) -> anyhow::Result<()> {
    loop {
        match watcher.receiver.recv().await {
            Some(Ok(event)) => watcher.process_event(event).await,
            Some(Err(e)) => {
                println!(
                    "{}",
                    l2(
                        "watch-error", /*"watch error: {:?}" */
                        "err_code",
                        "GR5",
                        "error",
                        e.to_string()
                    ));
                Ok(())
            },
            None => {
                break;
            }
        }?;
    }
    assert!(false, "watch_for_new_sockets should never exit");
    Ok(())
}


// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_socket_dir_watcher() {
        let (sender, mut receiver) = mpsc::channel(8);
        // Delete previous test socket dir if it exists
        let _ = tokio::fs::remove_dir_all("/tmp/socket_dir").await;
        tokio::fs::create_dir_all("/tmp/socket_dir").await.unwrap();
        let watcher = SocketDirWatcher::new(sender, "/tmp/socket_dir".to_string()).unwrap();

        tokio::spawn(watch_for_new_sockets(watcher));
        // Sleep to give the watcher time to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        // Create new socket directory
        tokio::fs::create_dir_all("/tmp/socket_dir/new-socket-dir").await.unwrap();
        let event = receiver.recv().await.unwrap();


        assert_eq!(event, UnixSocketEvent::NewSocket("ipc:///tmp/socket_dir/new-socket-dir/PAIR.zmq".to_string()));
    }
}
