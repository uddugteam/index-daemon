use crate::worker::worker::Worker;
use std::sync::mpsc;

#[macro_use]
extern crate clap;
use clap::App;

mod worker;

fn get_config_file_path(key: &str) -> Option<String> {
    let yaml = load_yaml!("../resources/cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    matches.value_of(key).map(|v| v.to_string())
}

fn get_config(key: &str) -> config::Config {
    let mut market_config = config::Config::default();

    if let Some(path) = get_config_file_path(key) {
        market_config.merge(config::File::with_name(&path)).unwrap();
    } else {
        let env_key = "APP_".to_string() + &key.to_uppercase();

        market_config
            .merge(config::Environment::with_prefix(&env_key).separator("_"))
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

fn main() {
    let market_config = get_config("market_config");

    let markets: Option<Vec<String>> =
        get_param_value_as_vec_of_string(&market_config, "exchanges");
    let markets: Option<Vec<&str>> = markets
        .as_ref()
        .map(|v| v.iter().map(|v| v.as_str()).collect());

    let coins: Option<Vec<String>> = get_param_value_as_vec_of_string(&market_config, "coins");
    let coins: Option<Vec<&str>> = coins
        .as_ref()
        .map(|v| v.iter().map(|v| v.as_str()).collect());

    let (tx, rx) = mpsc::channel();
    let worker = Worker::new(tx);
    worker.lock().unwrap().start(markets, coins);

    for received_thread in rx {
        let _ = received_thread.join();
    }
}
