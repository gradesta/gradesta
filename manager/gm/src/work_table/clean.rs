// TODO
/*
use std:fs;
use sdd:path;

extern fn clean(socket_dir: &String, term_timeout: u64){
    /// Sends the TERM signal to all connected services
    /// If they don't exit before `term_timeout`, sends the KILL signal
    let mut done: bool = true;
    for dir in fs::read_dir(socket_dir).unwrap() {
        done = false;
        https://doc.rust-lang.org/std/fs/fn.remove_file.html
        https://doc.rust-lang.org/std/fs/fn.remove_dir.html
    }
    // https://docs.rs/nix/0.23.0/nix/sys/signal/fn.kill.html
    if !done {
        clean(socket_dir, term_timeout);
    }
}
*/
