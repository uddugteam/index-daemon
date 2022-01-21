use clap::{App, Arg};
use env_logger::Builder;

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
    let mut config = config::Config::default();

    if let Some(path) = get_config_file_path(key) {
        config.merge(config::File::with_name(&path)).unwrap();
    } else {
        let env_key = "APP__".to_string() + &key.to_uppercase() + "_";

        config
            .merge(config::Environment::with_prefix(&env_key).separator("__"))
            .unwrap();
    }

    config
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

fn set_log_level(service_config: &config::Config) {
    let log_level = service_config
        .get_str("log_level")
        .unwrap_or("trace".to_string());

    let mut builder = Builder::from_default_env();
    builder.filter(Some("index_daemon"), log_level.parse().unwrap());
    builder.init();
}

pub struct ConfigScheme {
    pub market: MarketConfig,
    pub service: ServiceConfig,
}
impl ConfigScheme {
    pub fn new() -> Self {
        Self {
            market: MarketConfig::new(),
            service: ServiceConfig::new(),
        }
    }
}

pub struct MarketConfig {
    pub markets: Option<Vec<String>>,
    pub coins: Option<Vec<String>>,
    pub channels: Option<Vec<String>>,
}
impl MarketConfig {
    pub fn new() -> Self {
        let market_config = get_config("market_config");

        let markets = get_param_value_as_vec_of_string(&market_config, "exchanges");
        let coins = get_param_value_as_vec_of_string(&market_config, "coins");
        let channels = get_param_value_as_vec_of_string(&market_config, "channels");

        Self {
            markets,
            coins,
            channels,
        }
    }
}

pub struct ServiceConfig {
    pub rest_timeout_sec: u64,
    pub ws: bool,
    pub ws_host: String,
    pub ws_port: String,
    pub ws_answer_timeout_ms: u64,
}
impl ServiceConfig {
    pub fn new() -> Self {
        let service_config = get_config("service_config");

        set_log_level(&service_config);

        let rest_timeout_sec = service_config
            .get_str("rest_timeout_sec")
            .map(|v| v.parse().unwrap())
            .unwrap_or(1);
        if rest_timeout_sec < 1 {
            panic!(
                "Got wrong config value. service_config: rest_timeout_sec={}",
                rest_timeout_sec
            );
        }

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
            && (service_config.get_str("ws_host").is_ok()
                || service_config.get_str("ws_port").is_ok()
                || service_config.get_str("ws_answer_timeout_ms").is_ok())
        {
            panic!(
                "Got unexpected config. service_config: ws_*. That config is allowed only if ws=1"
            );
        }

        let ws_host = service_config
            .get_str("ws_host")
            .unwrap_or("127.0.0.1".to_string());
        let ws_port = service_config
            .get_str("ws_port")
            .unwrap_or("8080".to_string());
        let ws_answer_timeout_ms = service_config
            .get_str("ws_answer_timeout_ms")
            .map(|v| v.parse().unwrap())
            .unwrap_or(100);
        if ws_answer_timeout_ms < 100 {
            panic!(
                "Got wrong config value. Value is less than allowed min. service_config: ws_answer_timeout_ms={}",
                ws_answer_timeout_ms
            );
        }

        Self {
            rest_timeout_sec,
            ws,
            ws_host,
            ws_port,
            ws_answer_timeout_ms,
        }
    }
}
