use super::configuration::Configuration;
use clap::{Arg, Command, crate_version};
use humantime::Duration;
use std::path::PathBuf;
use std::process;

pub fn parse_args_and_environment() -> Configuration {
    let default_sockets_dir: &str = &format!(
        "{}/.cache/gradsta/services/",
        std::env::var("HOME").unwrap()
    );
    let default_heartbeat_timeout: humantime::Duration = std::time::Duration::new(15, 0).into();
    let matches = Command::new("gradesta manager")
        .about("Connects clients and services via websockets and ZMQ unix PAIR sockets. Evaluates walk trees.")
        .version(crate_version!())
        .arg(Arg::new("sockets_dir")
             .long("sockets-dir")
             .takes_value(true)
             .help("Directory where ZMQ unix sockets are found"))
        .arg(Arg::new("init")
             .long("init")
             .takes_value(true)
             .help("Init binary for launching services after manager startup"))
        .arg(Arg::new("port")
             .short('p')
             .long("port")
             .takes_value(true)
             .help("Port for websockets to connect"))
        .arg(Arg::new("service_heartbeat_timeout")
             .long("heartbeat_timeout")
             .takes_value(true)
             .help("Amount of time to wait after heartbeat before closing connection"))
        .get_matches();
    return Configuration {
        sockets_dir: match matches.value_of_t("sockets_dir") {
            Ok(value) => value,
            Err(EmptyValue) => default_sockets_dir.into(),
            Err(error) => {
                eprintln!("Could not parse sockets_dir: {}", error);
                process::exit(1);
                unreachable!()
            }
        },
        port: match matches.value_of_t("port") {
            Ok(value) => value,
            Err(EmptyValue) => 443,
            Err(error) => {
                eprintln!("Could not parse port number: {}", error);
                process::exit(1);
                unreachable!()
            }
        },
        init: match matches.value_of_t::<PathBuf>("init") {
            Ok(value) => Some(value),
            Err(EmptyValue) => Option::None,
            Err(error) => {
                eprintln!("Could not parse init: {}", error);
                process::exit(1);
                unreachable!()
            }
        },
        service_heartbeat_timeout:  match matches.value_of_t("service_heartbeat_timeout") {
            Ok(value) => value,
            Err(EmptyValue) => default_heartbeat_timeout,
            Err(error) => {
                eprintln!("Could not parse heartbeat timeout: {}", error);
                process::exit(1);
                unreachable!()
            }
        },
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Component};
    use humantime::Duration;

    #[test]
    fn test_default_values() {
        let config = parse_args_and_environment();
        assert_eq!(config.port, 443);
        assert_eq!(config.init, Option::None);
        assert_eq!(config.sockets_dir.components().last().unwrap(), Component::Normal("services".as_ref()));
        assert_eq!(config.service_heartbeat_timeout, "15sec".parse::<humantime::Duration>().unwrap());
    }
}
