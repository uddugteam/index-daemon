use chrono::Utc;
use rustc_serialize::json::Json;

use crate::worker::market_helpers::market::{
    parse_str_from_json_array, parse_str_from_json_object, Market,
};
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Okcoin {
    pub spine: MarketSpine,
}

impl Market for Okcoin {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn make_pair(&self, pair: (&str, &str)) -> String {
        (self.spine.get_masked_value(pair.0).to_string()
            + "-"
            + self.spine.get_masked_value(pair.1))
        .to_uppercase()
    }

    fn get_channel_text_view(&self, channel: MarketChannels) -> String {
        match channel {
            MarketChannels::Ticker => "ticker",
            MarketChannels::Trades => "trade",
            MarketChannels::Book => "depth_l2_tbt",
            // MarketChannels::Book => "depth",
        }
        .to_string()
    }

    fn get_websocket_url(&self, _pair: &str, _channel: MarketChannels) -> String {
        "wss://real.okcoin.com:8443/ws/v3".to_string()
    }

    fn get_websocket_on_open_msg(&self, pair: &str, channel: MarketChannels) -> Option<String> {
        Some(format!(
            "{{\"op\": \"subscribe\", \"args\": [\"spot/{}:{}\"]}}",
            self.get_channel_text_view(channel),
            pair
        ))
    }

    fn parse_ticker_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(array) = json.as_object().unwrap().get("data") {
                for object in array.as_array().unwrap() {
                    let object = object.as_object().unwrap();

                    // TODO: Check whether key `base_volume_24h` is right
                    let volume: f64 =
                        parse_str_from_json_object(object, "base_volume_24h").unwrap();

                    info!("new {} ticker on Okcoin with volume: {}", pair, volume);

                    let conversion_coef: f64 = self.spine.get_conversion_coef(&pair);
                    self.spine.set_total_volume(&pair, volume * conversion_coef);
                }
            }
        }
    }

    fn parse_last_trade_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(array) = json.as_object().unwrap().get("data") {
                for object in array.as_array().unwrap() {
                    let object = object.as_object().unwrap();

                    let last_trade_volume: f64 =
                        parse_str_from_json_object(object, "size").unwrap();
                    let mut last_trade_price: f64 =
                        parse_str_from_json_object(object, "price").unwrap();

                    let trade_type = object.get("side").unwrap().as_string().unwrap();
                    // TODO: Check whether inversion is right
                    if trade_type == "sell" {
                        // sell
                        last_trade_price *= -1.0;
                    } else if trade_type == "buy" {
                        // buy
                    }

                    info!(
                        "new {} trade on Okcoin with volume: {}, price: {}",
                        pair, last_trade_volume, last_trade_price,
                    );

                    let conversion_coef: f64 = self.spine.get_conversion_coef(&pair);
                    self.spine.set_last_trade_volume(&pair, last_trade_volume);
                    self.spine
                        .set_last_trade_price(&pair, last_trade_price * conversion_coef);
                }
            }
        }
    }

    fn parse_depth_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(object) = json.as_object() {
                if let Some(array) = object.get("data") {
                    if object.get("action").unwrap().as_string().unwrap() == "partial" {
                        for object in array.as_array().unwrap() {
                            let object = object.as_object().unwrap();

                            if let Some(asks) = object.get("asks") {
                                if let Some(bids) = object.get("bids") {
                                    let conversion_coef: f64 =
                                        self.spine.get_conversion_coef(&pair);

                                    let mut ask_sum: f64 = 0.0;
                                    for ask in asks.as_array().unwrap() {
                                        if let Some(ask) = ask.as_array() {
                                            let size: f64 =
                                                parse_str_from_json_array(ask, 1).unwrap();
                                            ask_sum += size;
                                        }
                                    }
                                    self.spine.set_total_ask(&pair, ask_sum);

                                    let mut bid_sum: f64 = 0.0;
                                    for bid in bids.as_array().unwrap() {
                                        if let Some(bid) = bid.as_array() {
                                            let price: f64 =
                                                parse_str_from_json_array(bid, 0).unwrap();
                                            let size: f64 =
                                                parse_str_from_json_array(bid, 1).unwrap();
                                            bid_sum += size * price;
                                        }
                                    }
                                    bid_sum *= conversion_coef;
                                    self.spine.set_total_bid(&pair, bid_sum);

                                    info!(
                                        "new {} book on Okcoin with ask_sum: {}, bid_sum: {}",
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
    }
}
