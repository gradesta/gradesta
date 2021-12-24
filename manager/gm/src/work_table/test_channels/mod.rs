#[cfg(test)]
use zmq;

const DEFAULT_SOCKET_PATH: &str = "ipc:///tmp/rust_test_channels_socket.ZMQ_REQ_REP";

#[cfg(not(test))]
pub fn report_to_custom_socket(socket_path: &str) {}

#[cfg(test)]
pub fn report_to_custom_socket(message: &str, socket_path: &str) {
    let ctx = zmq::Context::new();
    let socket = ctx.socket(zmq::REQ).unwrap();
    socket.connect(&socket_path).unwrap();
    socket.send(&message, 0).unwrap();
    socket.recv_msg(0).unwrap();
}

#[cfg(test)]
pub fn report(message: &str) {
    report_to_custom_socket(message, DEFAULT_SOCKET_PATH);
}

#[cfg(not(test))]
pub fn report(message: &str) {}

#[cfg(test)]
pub fn open_test_channel_with_custom_socket(socket_path: &str) ->  zmq::Socket {
    let ctx = zmq::Context::new();
    let socket = ctx.socket(zmq::REP).unwrap();
    socket.bind(&socket_path).unwrap();
    return socket;
}

#[cfg(test)]
pub fn open_test_channel() -> zmq::Socket {
    open_test_channel_with_custom_socket(DEFAULT_SOCKET_PATH)
}

pub fn expect(message: &str, socket: &zmq::Socket) {
    loop {
        let incoming: String = socket.recv_string(0).unwrap().unwrap();
        if incoming == message {
            return
        }
        println!("Ignoring incoming message:\n {}", incoming);
        send_continue(socket);
    }
}

pub fn send_continue(socket: &zmq::Socket) {
    socket.send("", 0).unwrap();
}

pub fn clear_expectations(socket: &zmq::Socket) {
    expect("END", socket);
    send_continue(&socket);
}

#[cfg(test)]
mod tests {
    use super::*;
    const MAGIC_TEST_FILE: &str = "/tmp/rust_test_channels_magic_test_file";

    #[test]
    fn test_test_channels() {
        use std::thread;
        use std::fs;
        use std::path;
        fs::remove_file(MAGIC_TEST_FILE).unwrap_or_default();
        let handle = thread::spawn(|| {
            let channel = open_test_channel();
            expect(&"foo", &channel);
            send_continue(&channel);
            expect(&"bar", &channel);
            fs::write(MAGIC_TEST_FILE, "foo").unwrap();
            send_continue(&channel);
            clear_expectations(&channel);
        });
        report(&"foo");
        report(&"baz");
        assert!(!path::Path::new(MAGIC_TEST_FILE).exists());
        report(&"bar");
        assert!(path::Path::new(MAGIC_TEST_FILE).exists());
        report(&"lol");
        report(&"END");
        handle.join().unwrap();
    }
}