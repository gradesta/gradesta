fn main() {
    use clap::{crate_version, Arg, Command};
    let matches = Command::new("test socket holder")
        .about("Bind a socket and hangs. Prints 'Ready!\n' to stdout when ready and 'TERM\n' to stdout when it gets the TERM signal. Make sure you clean up the socket after running your tests as this will not do any clean up what-so-ever.")
        .version(crate_version!())
        .arg(Arg::new("socket")
             .takes_value(true)
             .help("Socket to bind to"))
        .get_matches();

    let ctx = zmq::Context::new();
    let socket = ctx.socket(zmq::REP).unwrap();
    let socket_url: String = matches.value_of_t("socket").unwrap();
    socket.bind(&socket_url).unwrap();
    use signal_hook::{iterator::Signals, consts::signal::SIGTERM};
    let mut signals = Signals::new(&[SIGTERM]).unwrap();
    println!("Ready!");
    for _ in signals.forever() {
        println!("TERM");
    }
}
