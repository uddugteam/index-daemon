use crate::config_scheme::helper_functions::{
    get_config_from_config_files, get_default_data_expire_sec, get_default_data_expire_string,
    get_default_historical, get_default_host, get_default_percent_change_interval_sec,
    get_default_percent_change_interval_string, get_default_port, get_default_storage,
    set_log_level,
};
use crate::config_scheme::storage::Storage;
use clap::ArgMatches;
use parse_duration::parse;

#[derive(Clone)]
pub struct ServiceConfig {
    pub rest_timeout_sec: u64,
    pub ws: bool,
    pub ws_addr: String,
    pub ws_answer_timeout_ms: u64,
    pub storage: Option<Storage>,
    pub historical_storage_frequency_ms: u64,
    pub data_expire_sec: u64,
    pub percent_change_interval_sec: u64,
}

impl ServiceConfig {
    pub fn new(matches: &ArgMatches) -> Self {
        let default = Self::default();
        let service_config = get_config_from_config_files(matches, "service_config");

        set_log_level(&service_config);

        let rest_timeout_sec = service_config
            .get_str("rest_timeout_sec")
            .map(|v| v.parse().unwrap())
            .unwrap_or(default.rest_timeout_sec);
        assert!(rest_timeout_sec > 0);

        let ws = if let Ok(ws) = service_config.get_str("ws") {
            assert_eq!(ws, "1");

            true
        } else {
            default.ws
        };
        let unexpected_configs = ["ws_host", "ws_port", "ws_answer_timeout_ms"];
        if !ws {
            for unexpected_config in unexpected_configs {
                if service_config.get_str(unexpected_config).is_ok() {
                    panic!(
                        "Got unexpected config. service_config: {}. That config is allowed only if ws=1", unexpected_config,
                    );
                }
            }
        }

        let ws_host = service_config
            .get_str("ws_host")
            .unwrap_or(get_default_host());
        let ws_port = service_config
            .get_str("ws_port")
            .unwrap_or(get_default_port());
        let ws_addr = ws_host + ":" + &ws_port;
        let ws_answer_timeout_ms = service_config
            .get_str("ws_answer_timeout_ms")
            .map(|v| v.parse().unwrap())
            .unwrap_or(default.ws_answer_timeout_ms);
        assert!(ws_answer_timeout_ms >= 100);

        let historical = if let Ok(historical) = service_config.get_str("historical") {
            assert_eq!(historical, "1");

            true
        } else {
            get_default_historical()
        };
        let unexpected_configs = ["storage", "historical_storage_frequency_ms", "data_expire"];
        if !historical {
            for unexpected_config in unexpected_configs {
                if service_config.get_str(unexpected_config).is_ok() {
                    panic!(
                        "Got unexpected config. service_config: {}. These configs are allowed only if historical=1", unexpected_config,
                    );
                }
            }
        }

        let storage = if historical {
            Some(
                service_config
                    .get_str("storage")
                    .map(|v| Storage::from_str(&v))
                    .unwrap_or_default(),
            )
        } else {
            None
        };
        let historical_storage_frequency_ms = service_config
            .get_str("historical_storage_frequency_ms")
            .map(|v| v.parse().unwrap())
            .unwrap_or(default.historical_storage_frequency_ms);
        assert!(historical_storage_frequency_ms >= 10);

        let data_expire_sec = parse(
            &service_config
                .get_str("data_expire")
                .unwrap_or(get_default_data_expire_string()),
        )
        .unwrap()
        .as_secs();
        assert!(data_expire_sec > 0);

        let percent_change_interval_sec = parse(
            &service_config
                .get_str("percent_change_interval")
                .unwrap_or(get_default_percent_change_interval_string()),
        )
        .unwrap()
        .as_secs();
        assert!(percent_change_interval_sec > 0);

        Self {
            rest_timeout_sec,
            ws,
            ws_addr,
            ws_answer_timeout_ms,
            storage,
            historical_storage_frequency_ms,
            data_expire_sec,
            percent_change_interval_sec,
        }
    }
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            rest_timeout_sec: 1,
            ws: false,
            ws_addr: get_default_host() + ":" + &get_default_port(),
            ws_answer_timeout_ms: 100,
            storage: get_default_storage(get_default_historical()),
            historical_storage_frequency_ms: 20,
            data_expire_sec: get_default_data_expire_sec(),
            percent_change_interval_sec: get_default_percent_change_interval_sec(),
        }
    }
}
