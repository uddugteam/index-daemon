use crate::config_scheme::helper_functions::{get_config, set_log_level};

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
