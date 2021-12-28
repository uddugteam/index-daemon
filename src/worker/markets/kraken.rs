use rustc_serialize::json::Json;

use crate::worker::market_helpers::market::{depth_helper_v1, parse_str_from_json_array, Market};
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Kraken {
    pub spine: MarketSpine,
}

impl Market for Kraken {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn make_pair(&self, pair: (&str, &str)) -> String {
        (self.spine.get_masked_value(pair.0).to_string()
            + "/"
            + self.spine.get_masked_value(pair.1))
        .to_uppercase()
    }

    fn get_channel_text_view(&self, channel: MarketChannels) -> String {
        match channel {
            MarketChannels::Ticker => "ticker",
            MarketChannels::Trades => "trade",
            MarketChannels::Book => "book",
        }
        .to_string()
    }

    fn get_websocket_url(&self, _pair: &str, _channel: MarketChannels) -> String {
        "wss://ws.kraken.com".to_string()
    }

    fn get_websocket_on_open_msg(&self, pair: &str, channel: MarketChannels) -> Option<String> {
        Some(format!(
            "{{\"event\": \"subscribe\", \"pair\": [\"{}\"], \"subscription\": {{\"name\": \"{}\"}}}}",
            pair, self.get_channel_text_view(channel)
        ))
    }

    fn parse_ticker_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(array) = json.as_array() {
                if let Some(array) = array[1].as_object().unwrap().get("v").unwrap().as_array() {
                    let volume: f64 = parse_str_from_json_array(array, 1).unwrap();

                    self.parse_ticker_info_inner(pair, volume);
                }
            }
        }
    }

    fn parse_last_trade_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(array) = json.as_array() {
                for array in array[1].as_array().unwrap() {
                    let array = array.as_array().unwrap();

                    let mut last_trade_price: f64 = parse_str_from_json_array(array, 0).unwrap();
                    let last_trade_volume: f64 = parse_str_from_json_array(array, 1).unwrap();

                    let trade_type = array[3].as_string().unwrap();
                    // TODO: Check whether inversion is right
                    if trade_type == "s" {
                        // sell
                        last_trade_price *= -1.0;
                    } else if trade_type == "b" {
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

    fn parse_depth_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(array) = json.as_array() {
                if let Some(object) = array[1].as_object() {
                    if let Some(asks) = object.get("as") {
                        if let Some(bids) = object.get("bs") {
                            let asks = depth_helper_v1(asks);
                            let bids = depth_helper_v1(bids);

                            self.parse_depth_info_inner(pair, asks, bids);
                        }
                    }
                }
            }
        }
    }
}
