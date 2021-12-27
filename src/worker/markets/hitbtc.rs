use chrono::Utc;
use rustc_serialize::json::Json;

use crate::worker::market_helpers::market::{
    parse_str_from_json_array, parse_str_from_json_object, Market,
};
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Hitbtc {
    pub spine: MarketSpine,
}

impl Market for Hitbtc {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn make_pair(&self, pair: (&str, &str)) -> String {
        (self.spine.get_masked_value(pair.0).to_string() + self.spine.get_masked_value(pair.1))
            .to_uppercase()
    }

    fn get_channel_text_view(&self, channel: MarketChannels) -> String {
        match channel {
            MarketChannels::Ticker => "ticker/1s",
            MarketChannels::Trades => "trades",
            MarketChannels::Book => "orderbook/full",
        }
        .to_string()
    }

    fn get_websocket_url(&self, _pair: &str, _channel: MarketChannels) -> String {
        "wss://api.hitbtc.com/api/3/ws/public".to_string()
    }

    fn get_websocket_on_open_msg(&self, pair: &str, channel: MarketChannels) -> Option<String> {
        Some(format!(
            "{{\"method\": \"subscribe\", \"ch\": \"{}\", \"params\": {{\"symbols\": [\"{}\"]}} }}",
            self.get_channel_text_view(channel),
            pair
        ))
    }

    fn parse_ticker_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(object) = json.as_object().unwrap().get("data") {
                if let Some(object) = object.as_object().unwrap().get(&pair).unwrap().as_object() {
                    let volume: f64 = parse_str_from_json_object(object, "v").unwrap();

                    info!("new {} ticker on Hitbtc with volume: {}", pair, volume);

                    let conversion_coef: f64 = self.spine.get_conversion_coef(&pair);
                    self.spine.set_total_volume(&pair, volume * conversion_coef);
                }
            }
        }
    }

    fn parse_last_trade_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(object) = json.as_object().unwrap().get("update") {
                if let Some(array) = object.as_object().unwrap().get(&pair).unwrap().as_array() {
                    for object in array {
                        let object = object.as_object().unwrap();

                        let last_trade_volume: f64 =
                            parse_str_from_json_object(object, "q").unwrap();
                        let mut last_trade_price: f64 =
                            parse_str_from_json_object(object, "p").unwrap();

                        let trade_type = object.get("s").unwrap().as_string().unwrap();
                        // TODO: Check whether inversion is right
                        if trade_type == "sell" {
                            // sell
                            last_trade_price *= -1.0;
                        } else if trade_type == "buy" {
                            // buy
                        }

                        info!(
                            "new {} trade on Hitbtc with volume: {}, price: {}",
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
    }

    fn parse_depth_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(object) = json.as_object().unwrap().get("snapshot") {
                if let Some(object) = object.as_object().unwrap().get(&pair).unwrap().as_object() {
                    if let Some(asks) = object.get("a") {
                        if let Some(bids) = object.get("b") {
                            let conversion_coef: f64 = self.spine.get_conversion_coef(&pair);

                            let mut ask_sum: f64 = 0.0;
                            for ask in asks.as_array().unwrap() {
                                if let Some(ask) = ask.as_array() {
                                    let size: f64 = parse_str_from_json_array(ask, 1).unwrap();
                                    ask_sum += size;
                                }
                            }
                            self.spine.set_total_ask(&pair, ask_sum);

                            let mut bid_sum: f64 = 0.0;
                            for bid in bids.as_array().unwrap() {
                                if let Some(bid) = bid.as_array() {
                                    let price: f64 = parse_str_from_json_array(bid, 0).unwrap();
                                    let size: f64 = parse_str_from_json_array(bid, 1).unwrap();
                                    bid_sum += size * price;
                                }
                            }
                            bid_sum *= conversion_coef;
                            self.spine.set_total_bid(&pair, bid_sum);

                            info!(
                                "new {} book on Hitbtc with ask_sum: {}, bid_sum: {}",
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
