use regex::Regex;
use rustc_serialize::json::Json;

use crate::worker::market_helpers::market::Market;
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Huobi {
    pub spine: MarketSpine,
}

impl Huobi {
    fn depth_helper(json: &Json) -> Vec<(f64, f64)> {
        json.as_array()
            .unwrap()
            .iter()
            .map(|v| {
                let v = v.as_array().unwrap();
                (v[0].as_f64().unwrap(), v[1].as_f64().unwrap())
            })
            .collect()
    }
}

impl Market for Huobi {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn make_pair(&self, pair: (&str, &str)) -> String {
        (self.spine.get_masked_value(pair.0).to_string() + self.spine.get_masked_value(pair.1))
            .to_lowercase()
    }

    fn get_channel_text_view(&self, channel: MarketChannels) -> String {
        match channel {
            MarketChannels::Ticker => "ticker",
            MarketChannels::Trades => "trade.detail",
            MarketChannels::Book => "depth.step0",
        }
        .to_string()
    }

    fn get_websocket_url(&self, _pair: &str, _channel: MarketChannels) -> String {
        "wss://api.huobi.pro/ws".to_string()
    }

    fn get_websocket_on_open_msg(&self, pair: &str, channel: MarketChannels) -> Option<String> {
        Some(format!(
            "{{\"sub\": \"market.{}.{}\"}}",
            pair,
            self.get_channel_text_view(channel)
        ))
    }

    fn parse_ticker_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(object) = json.as_object() {
                if let Some(object) = object.get("tick") {
                    if let Some(object) = object.as_object() {
                        let volume: f64 = object.get("vol").unwrap().as_f64().unwrap();

                        self.parse_ticker_info_inner(pair, volume);
                    }
                }
            }
        }
    }

    fn parse_last_trade_info(&mut self, pair: String, info: String) {
        let info = if Json::from_str(&info).is_ok() {
            info
        } else {
            // Error: invalid number (integer is too long).
            // Since we don't need its real value, we can to replace it with fake, but valid number.
            let regex_too_big_integer = Regex::new("^(.*)(\\D)(?P<n>\\d{18,})(\\D)(.*)$").unwrap();
            let too_big_integer = regex_too_big_integer.replace_all(&info, "$n").to_string();

            info.replace(&too_big_integer, "123456")
        };
        if let Ok(json) = Json::from_str(&info) {
            if let Some(object) = json.as_object().unwrap().get("tick") {
                if let Some(array) = object.as_object().unwrap().get("data") {
                    for object in array.as_array().unwrap() {
                        let object = object.as_object().unwrap();

                        let last_trade_volume: f64 =
                            object.get("amount").unwrap().as_f64().unwrap();
                        let mut last_trade_price: f64 =
                            object.get("price").unwrap().as_f64().unwrap();

                        let trade_type = object.get("direction").unwrap().as_string().unwrap();
                        // TODO: Check whether inversion is right
                        if trade_type == "sell" {
                            // sell
                            last_trade_price *= -1.0;
                        } else if trade_type == "buy" {
                            // buy
                        }

                        self.parse_last_trade_info_inner(
                            pair.clone(),
                            last_trade_volume,
                            last_trade_price,
                        );
                    }
                }
            }
        }
    }

    fn parse_depth_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(object) = json.as_object().unwrap().get("tick") {
                if let Some(object) = object.as_object() {
                    if let Some(asks) = object.get("asks") {
                        if let Some(bids) = object.get("bids") {
                            let asks = Self::depth_helper(asks);
                            let bids = Self::depth_helper(bids);

                            self.parse_depth_info_inner(pair, asks, bids);
                        }
                    }
                }
            }
        }
    }
}
