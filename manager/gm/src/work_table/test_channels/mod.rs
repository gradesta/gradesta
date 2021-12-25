const DEFAULT_SOCKET_PATH: &str = "/tmp/rust_test_channels_socket.ZMQ_REQ_REP";

#[cfg(not(test))]
pub fn report_to_custom_socket(socket_path: &str) {}

#[cfg(test)]
pub fn report_to_custom_socket(message: &str, socket_path: &str) {
    use std::path;
    if !path::Path::new(socket_path).exists() {
        return; // Fall through if we're not expecting anything so that non-expectant tests work.
    }

    let ctx = zmq::Context::new();
    let socket = ctx.socket(zmq::REQ).unwrap();
    let socket_url = format!("ipc://{}", socket_path);
    socket.connect(&socket_url).unwrap();
    socket.send(&message, 0).unwrap();
    socket.recv_msg(0).unwrap();
}

#[cfg(test)]
pub fn report(message: &str) {
    report_to_custom_socket(message, &for_tests::get_default_socket_path());
}

#[cfg(not(test))]
pub fn report(message: &str) {}

#[cfg(test)]
pub mod for_tests {
    use super::*;
    use zmq;
    use std::path;
    use sha2::{Sha256, Digest};
    use hex;

    pub fn get_default_socket_path() -> String {
        let mut hasher = Sha256::new();
        hasher.update(std::thread::current().name().unwrap().clone().as_bytes());
        let thread_name_hash = hex::encode(hasher.finalize());
        format!("{}{}",
            DEFAULT_SOCKET_PATH,
            thread_name_hash,
        )
    }

    pub fn open_test_channel_with_custom_socket(socket_path: &str) -> zmq::Socket {
        assert!(!path::Path::new(socket_path).exists());
        let ctx = zmq::Context::new();
        let socket = ctx.socket(zmq::REP).unwrap();
        let socket_url = format!("ipc://{}", socket_path);
        socket.bind(&socket_url).unwrap();
        return socket;
    }

    #[cfg(test)]
    pub fn open_test_channel() -> zmq::Socket {
        open_test_channel_with_custom_socket(&get_default_socket_path())
    }

    pub fn expect(message: &str, socket: &zmq::Socket) {
        loop {
            let incoming: String = socket.recv_string(0).unwrap().unwrap();
            if incoming == message {
                return;
            }
            println!("Ignoring incoming message:\n {}", incoming);
            send_continue(socket);
        }
    }

    pub fn send_continue(socket: &zmq::Socket) {
        socket.send("", 0).unwrap();
    }

    pub fn clear_expectations(socket: zmq::Socket) {
        expect("END", &socket);
        send_continue(&socket);
        let url = socket.get_last_endpoint().unwrap().unwrap();
        socket.disconnect(&url).unwrap();
        drop(socket);
        use std::fs;
        fs::remove_file(&url[6..]).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::for_tests::*;
    use super::*;
    const MAGIC_TEST_FILE: &str = "/tmp/rust_test_channels_magic_test_file";

    #[test]
    fn test_test_channels() {
        use std::fs;
        use std::path;
        assert!(!path::Path::new(&get_default_socket_path()).exists());
        report(&"ignored");
        fs::remove_file(MAGIC_TEST_FILE).unwrap_or_default();
        use std::thread;
        let channel = open_test_channel();
        let handle = thread::spawn(|| {
            expect(&"foo", &channel);
            send_continue(&channel);
            expect(&"bar", &channel);
            fs::write(MAGIC_TEST_FILE, "foo").unwrap();
            send_continue(&channel);
            clear_expectations(channel);
        });
        report(&"foo");
        assert!(path::Path::new(&get_default_socket_path()).exists());
        report(&"baz");
        assert!(!path::Path::new(MAGIC_TEST_FILE).exists());
        report(&"bar");
        assert!(path::Path::new(MAGIC_TEST_FILE).exists());
        report(&"lol");
        report(&"END");
        handle.join().unwrap();
        assert!(!path::Path::new(&get_default_socket_path()).exists());
        fs::remove_file(MAGIC_TEST_FILE).unwrap();
        assert!(!path::Path::new(MAGIC_TEST_FILE).exists());
    }
}
