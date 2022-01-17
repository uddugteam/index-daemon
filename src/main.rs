use clap::{App, Arg};
use env_logger::Builder;
use std::sync::mpsc;

use crate::worker::worker::Worker;

#[macro_use]
extern crate log;

mod repository;
mod worker;

fn get_config_file_path(key: &str) -> Option<String> {
    let matches = App::new("ICEX")
        .version("1.0")
        .arg(
            Arg::with_name("service_config")
                .long("service_config")
                .value_name("PATH")
                .help("Service config file path")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("market_config")
                .long("market_config")
                .value_name("PATH")
                .help("Market config file path")
                .takes_value(true),
        )
        .get_matches();

    matches.value_of(key).map(|v| v.to_string())
}

fn get_config(key: &str) -> config::Config {
    let mut market_config = config::Config::default();

    if let Some(path) = get_config_file_path(key) {
        market_config.merge(config::File::with_name(&path)).unwrap();
    } else {
        let env_key = "APP__".to_string() + &key.to_uppercase() + "_";

        market_config
            .merge(config::Environment::with_prefix(&env_key).separator("__"))
            .unwrap();
    }

    market_config
}

fn get_param_value_as_vec_of_string(config: &config::Config, key: &str) -> Option<Vec<String>> {
    if let Ok(string) = config.get_str(key) {
        Some(string.split(',').map(|v| v.to_string()).collect())
    } else {
        config
            .get_array(key)
            .ok()
            .map(|v| v.into_iter().map(|v| v.into_str().unwrap()).collect())
    }
}

fn get_all_configs() -> (
    Option<Vec<String>>,
    Option<Vec<String>>,
    Option<Vec<String>>,
    Option<u64>,
    bool,
    String,
    String,
) {
    let service_config = get_config("service_config");

    let log_level = service_config
        .get_str("log_level")
        .unwrap_or("trace".to_string());

    let rest_timeout_sec = service_config
        .get_str("rest_timeout_sec")
        .map(|v| v.parse().unwrap())
        .ok();

    let ws = if let Ok(ws) = service_config.get_str("ws") {
        if ws == "1" {
            true
        } else {
            panic!("Got wrong config value. service_config: ws={}", ws);
        }
    } else {
        false
    };
    if !ws
        && (service_config.get_str("ws_host").is_ok() || service_config.get_str("ws_port").is_ok())
    {
        panic!(
            "Got unexpected config. service_config: ws_host. That config is allowed only if ws=1"
        );
    }

    let ws_host = service_config
        .get_str("ws_host")
        .unwrap_or("127.0.0.1".to_string());
    let ws_port = service_config
        .get_str("ws_port")
        .unwrap_or("8080".to_string());

    let mut builder = Builder::from_default_env();
    builder.filter(Some("index_daemon"), log_level.parse().unwrap());
    builder.init();

    let market_config = get_config("market_config");

    let markets = get_param_value_as_vec_of_string(&market_config, "exchanges");
    let coins = get_param_value_as_vec_of_string(&market_config, "coins");
    let channels = get_param_value_as_vec_of_string(&market_config, "channels");

    (
        markets,
        coins,
        channels,
        rest_timeout_sec,
        ws,
        ws_host,
        ws_port,
    )
}

fn main() {
    let (markets, coins, channels, rest_timeout_sec, ws, ws_host, ws_port) = get_all_configs();

    let (tx, rx) = mpsc::channel();
    let worker = Worker::new(tx);
    worker.lock().unwrap().start(
        markets,
        coins,
        channels,
        rest_timeout_sec,
        ws,
        ws_host,
        ws_port,
    );

    for received_thread in rx {
        let _ = received_thread.join();
    }
}
