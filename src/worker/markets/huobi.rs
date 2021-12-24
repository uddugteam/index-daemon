use chrono::Utc;
use regex::Regex;
use rustc_serialize::json::Json;

use crate::worker::market_helpers::market::{
    parse_str_from_json_array, parse_str_from_json_object, Market,
};
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Huobi {
    pub spine: MarketSpine,
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

                        info!("new {} ticker on Huobi with volume: {}", pair, volume);

                        let conversion_coef: f64 = self.spine.get_conversion_coef(&pair);
                        self.spine.set_total_volume(&pair, volume * conversion_coef);
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
                    let conversion_coef: f64 = self.spine.get_conversion_coef(&pair);

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

                        info!(
                            "new {} trade on Huobi with volume: {}, price: {}",
                            pair, last_trade_volume, last_trade_price,
                        );

                        self.spine.set_last_trade_volume(&pair, last_trade_volume);
                        self.spine
                            .set_last_trade_price(&pair, last_trade_price * conversion_coef);
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
                            let conversion_coef: f64 = self.spine.get_conversion_coef(&pair);

                            let mut ask_sum: f64 = 0.0;
                            for ask in asks.as_array().unwrap() {
                                if let Some(ask) = ask.as_array() {
                                    let size: f64 = ask[1].as_f64().unwrap();
                                    ask_sum += size;
                                }
                            }
                            self.spine.set_total_ask(&pair, ask_sum);

                            let mut bid_sum: f64 = 0.0;
                            for bid in bids.as_array().unwrap() {
                                if let Some(bid) = bid.as_array() {
                                    let price: f64 = bid[0].as_f64().unwrap();
                                    let size: f64 = bid[1].as_f64().unwrap();
                                    bid_sum += size * price;
                                }
                            }
                            bid_sum *= conversion_coef;
                            self.spine.set_total_bid(&pair, bid_sum);

                            info!(
                                "new {} book on Huobi with ask_sum: {}, bid_sum: {}",
                                pair, ask_sum, bid_sum
                            );

                            let timestamp = Utc::now();
                            self.spine
                                .get_exchange_pairs_mut()
                                .get_mut(&pair)
                                .unwrap()
                                .set_timestamp(timestamp);
                        }
                    }
                }
            }
        }
    }
}
