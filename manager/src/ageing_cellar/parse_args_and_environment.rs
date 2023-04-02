use super::configuration::Configuration;
use anyhow::anyhow;
use clap::{crate_version, Arg, ArgAction, Command};
use std::path::PathBuf;

use super::localizer::*;

pub fn parse_args_and_environment() -> anyhow::Result<Configuration> {
    let default_sockets_dir: &str = &format!(
        "{}/.cache/gradesta/services/",
        std::env::var("HOME").unwrap()
    );
    let matches = Command::new(l("gradesta-command") /* "gradesta manager" */)
        .about(l("gradesta-command-about").as_ref() /* "Connects clients and services via websockets and ZMQ unix PAIR sockets. Evaluates walk trees." */)
        .version(crate_version!())
        .arg(Arg::new("sockets_dir")
             .long("sockets-dir")
             .takes_value(true)
             .help(l("gradesta-command-sockets-dir-help").as_ref() /* "Directory where ZMQ unix sockets are found" */)
             .default_value(default_sockets_dir.into()))
        .arg(Arg::new("init")
             .long("init")
             .takes_value(true)
             .help(l("gradesta-command-init-help").as_ref() /* "Init binary for launching services after manager startup" */))
        .arg(Arg::new("port")
             .short('p')
             .long("port")
             .takes_value(true)
             .help(l("gradesta-command-port-help").as_ref() /* "Port for websockets to connect" */)
             .default_value("443"))
        .arg(Arg::new("no-websockets")
             .long("no-websockets")
             .help(l("gradesta-command-no-websockets-help").as_ref() /* "Dissable websockets" */)
             .action(ArgAction::SetTrue)
        )
        .arg(Arg::new("no-unix-sockets")
             .long("no-unix-sockets")
             .takes_value(false)
             .help(l("gradesta-command-no-unix-sockets-help").as_ref() /* "Dissable unix sockets" */)
             .action(ArgAction::SetTrue)
        )
        .arg(Arg::new("service-heartbeat-timeout")
             .long("heartbeat-timeout")
             .takes_value(true)
             .help(l("gradesta-command-service-heartbeat-timeout-help").as_ref() /* "Amount of time to wait after heartbeat before closing connection" */)
             .default_value("15s"))
        .get_matches();
    let sockets_dir = if *matches.get_one::<bool>("no-unix-sockets").unwrap() {
        None
    } else {
        Some(matches.value_of_t("sockets_dir")?)
    };
    let port = if *matches.get_one::<bool>("no-websockets").unwrap() {
        None
    } else {
        Some(matches.value_of_t("port")?)
    };
    if sockets_dir == None && port == None {
        return Err(anyhow!(
            l1("gradesta-command-no-sockets-error", "err_code", "GR8") /* "Either UNIX sockets or websockets must be enabled." */
        ));
    }
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
    let service_heartbeat_timeout = matches.value_of_t("service-heartbeat-timeout")?;
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
        assert_eq!(config.port.unwrap(), 443);
        assert_eq!(config.init, Option::None);
        assert_eq!(
            config.sockets_dir.unwrap().components().last().unwrap(),
            Component::Normal("services".as_ref())
        );
        assert_eq!(
            config.service_heartbeat_timeout,
            "15sec".parse::<humantime::Duration>().unwrap()
        );
    }
}
