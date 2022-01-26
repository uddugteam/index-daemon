use crate::config_scheme::helper_functions::{
    get_config, get_default_host, get_default_port, set_log_level,
};

pub struct ServiceConfig {
    pub rest_timeout_sec: u64,
    pub ws: bool,
    pub ws_addr: String,
    pub ws_answer_timeout_ms: u64,
}
impl ServiceConfig {
    pub fn new() -> Self {
        let default = Self::default();
        let service_config = get_config("service_config");

        set_log_level(&service_config);

        let rest_timeout_sec = service_config
            .get_str("rest_timeout_sec")
            .map(|v| v.parse().unwrap())
            .unwrap_or(default.rest_timeout_sec);
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
            default.ws
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
            .unwrap_or(get_default_host());
        let ws_port = service_config
            .get_str("ws_port")
            .unwrap_or(get_default_port());
        let ws_addr = ws_host + ":" + &ws_port;
        let ws_answer_timeout_ms = service_config
            .get_str("ws_answer_timeout_ms")
            .map(|v| v.parse().unwrap())
            .unwrap_or(default.ws_answer_timeout_ms);
        if ws_answer_timeout_ms < 100 {
            panic!(
                "Got wrong config value. Value is less than allowed min. service_config: ws_answer_timeout_ms={}",
                ws_answer_timeout_ms
            );
        }

        Self {
            rest_timeout_sec,
            ws,
            ws_addr,
            ws_answer_timeout_ms,
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
        }
    }
}
