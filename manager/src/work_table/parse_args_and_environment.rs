use super::configuration::Configuration;
use anyhow::Result;
use clap::{crate_version, Arg, Command};
use std::path::PathBuf;

pub fn parse_args_and_environment() -> anyhow::Result<Configuration> {
    let default_sockets_dir: &str = &format!(
        "{}/.cache/gradsta/services/",
        std::env::var("HOME").unwrap()
    );
    let matches = Command::new("gradesta manager")
        .about("Connects clients and services via websockets and ZMQ unix PAIR sockets. Evaluates walk trees.")
        .version(crate_version!())
        .arg(Arg::new("sockets_dir")
             .long("sockets-dir")
             .takes_value(true)
             .help("Directory where ZMQ unix sockets are found")
             .default_value(default_sockets_dir.into()))
        .arg(Arg::new("init")
             .long("init")
             .takes_value(true)
             .help("Init binary for launching services after manager startup"))
        .arg(Arg::new("port")
             .short('p')
             .long("port")
             .takes_value(true)
             .help("Port for websockets to connect")
             .default_value("443"))
        .arg(Arg::new("service_heartbeat_timeout")
             .long("heartbeat_timeout")
             .takes_value(true)
             .help("Amount of time to wait after heartbeat before closing connection")
             .default_value("15s"))
        .get_matches();
    let sockets_dir = matches.value_of_t("sockets_dir")?;
    let port = matches.value_of_t("port")?;
    let init = match matches.value_of_t::<PathBuf>("init") {
        Ok(value) => Ok(Some(value)),
        Err(error) => {
            if error.kind() == clap::ErrorKind::ArgumentNotFound {
                Ok(None)
            } else {
                Err(error)
            }
        }
    }?;
    let service_heartbeat_timeout = matches.value_of_t("service_heartbeat_timeout")?;
    Ok(Configuration {
        sockets_dir: sockets_dir,
        port: port,
        init: init,
        service_heartbeat_timeout: service_heartbeat_timeout,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Component;

    #[test]
    fn test_default_values() {
        let config = parse_args_and_environment().unwrap();
        assert_eq!(config.port, 443);
        assert_eq!(config.init, Option::None);
        assert_eq!(
            config.sockets_dir.components().last().unwrap(),
            Component::Normal("services".as_ref())
        );
        assert_eq!(
            config.service_heartbeat_timeout,
            "15sec".parse::<humantime::Duration>().unwrap()
        );
    }
}
