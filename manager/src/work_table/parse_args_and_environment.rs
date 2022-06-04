use super::configuration::Configuration;
use clap::Parser;

pub fn parse_args_and_environment() -> Configuration {
    return Configuration::parse();
}
