use chrono::Utc;
use rustc_serialize::json::{Json, ToJson};
use std::collections::HashMap;

use crate::worker::defaults::POLONIEX_EXCHANGE_PAIRS;
use crate::worker::market_helpers::market::{parse_str_from_json_array, Market};
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Poloniex {
    pub spine: MarketSpine,
    pair_codes: HashMap<(String, String), String>,
}

impl Poloniex {
    pub fn new(spine: MarketSpine) -> Self {
        let mut pair_codes = HashMap::new();
        for (pair_code, pair_tuple) in POLONIEX_EXCHANGE_PAIRS {
            let pair: (String, String) = (
                spine.get_unmasked_value(pair_tuple.0).to_string(),
                spine.get_unmasked_value(pair_tuple.1).to_string(),
            );
            let pair2 = (pair.1.clone(), pair.0.clone());

            pair_codes.insert(pair, pair_code.to_string());
            pair_codes.insert(pair2, pair_code.to_string());
        }

        Self { spine, pair_codes }
    }

    fn coin_exists(&self, coin: &str) -> bool {
        let pair = (coin.to_string(), "USD".to_string());
        self.pair_codes.get(&pair).is_some()
    }
}

impl Market for Poloniex {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn make_pair(&self, pair: (&str, &str)) -> String {
        let pair = (pair.0.to_string(), pair.1.to_string());
        self.pair_codes.get(&pair).unwrap().clone()
    }

    fn get_channel_text_view(&self, channel: MarketChannels) -> String {
        match channel {
            MarketChannels::Ticker => "1003".to_string(),
            MarketChannels::Trades => {
                // There are no distinct Trades channel in Poloniex. We get Trades inside of Book channel.
                panic!("Poloniex: Subscription to wrong channel: Trades.")
            }
            MarketChannels::Book => {
                // This string was intentionally left blank, because Poloniex don't have code for Book
                // and we pass pair code instead of it (we do this in fn get_websocket_on_open_msg)
                "".to_string()
            }
        }
    }

    fn get_websocket_url(&self, _pair: &str, _channel: MarketChannels) -> String {
        "wss://api2.poloniex.com".to_string()
    }

    fn get_websocket_on_open_msg(&self, pair: &str, channel: MarketChannels) -> Option<String> {
        let channel_text_view = if let MarketChannels::Book = channel {
            pair.to_string()
        } else {
            self.get_channel_text_view(channel)
        };

        Some(format!(
            "{{\"command\": \"subscribe\", \"channel\": {}}}",
            channel_text_view,
        ))
    }

    fn parse_ticker_info(&mut self, _pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(array) = json.as_array().unwrap().get(2) {
                if let Some(object) = array.as_array().unwrap().get(2).unwrap().as_object() {
                    let volumes: HashMap<String, f64> = object
                        .iter()
                        .filter(|(k, _)| {
                            // Remove unknown coins
                            self.coin_exists(k)
                        })
                        .map(|(k, v)| {
                            // Convert key from coin name to pair code
                            // and convert value to f64
                            (
                                self.make_pair((k, "USD")),
                                v.as_string().unwrap().parse().unwrap(),
                            )
                        })
                        .filter(|(k, _)| {
                            // Remove unneeded pairs
                            self.spine.get_exchange_pairs().get(k).is_some()
                        })
                        .collect();

                    for (pair_code, volume) in volumes {
                        let pair_tuple = self.spine.get_pairs().get(&pair_code).unwrap();
                        info!(
                            "new {:?} ticker on Poloniex with volume: {}",
                            pair_tuple, volume
                        );

                        let conversion_coef: f64 = self.spine.get_conversion_coef(&pair_code);
                        self.spine
                            .set_total_volume(&pair_code, volume * conversion_coef);
                    }
                }
            }
        }
    }

    fn parse_last_trade_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(array) = json.as_array() {
                let mut last_trade_price: f64 = parse_str_from_json_array(array, 3).unwrap();
                let last_trade_volume: f64 = parse_str_from_json_array(array, 4).unwrap();

                let trade_type = array[2].as_u64().unwrap();
                // TODO: Check whether inversion is right
                if trade_type == 0 {
                    // sell
                    last_trade_price *= -1.0;
                } else if trade_type == 1 {
                    // buy
                }

                let pair_tuple = self.spine.get_pairs().get(&pair).unwrap();
                info!(
                    "new {:?} trade on Poloniex with volume: {}, price: {}",
                    pair_tuple, last_trade_volume, last_trade_price,
                );

                let conversion_coef: f64 = self.spine.get_conversion_coef(&pair);
                self.spine.set_last_trade_volume(&pair, last_trade_volume);
                self.spine
                    .set_last_trade_price(&pair, last_trade_price * conversion_coef);
            }
        }
    }

    fn parse_depth_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(json) = json.as_array().unwrap().get(2) {
                for array in json.as_array().unwrap() {
                    let array = array.as_array().unwrap();

                    if array[0].as_string().unwrap() == "i" {
                        // book
                        if let Some(object) = array.get(1).unwrap().as_object() {
                            if let Some(object) = object.get("orderBook") {
                                if let Some(array) = object.as_array() {
                                    let conversion_coef: f64 =
                                        self.spine.get_conversion_coef(&pair);

                                    let asks = array[0].as_object().unwrap();
                                    let mut ask_sum: f64 = 0.0;
                                    for size in asks.values() {
                                        let size: f64 = size.as_string().unwrap().parse().unwrap();

                                        ask_sum += size;
                                    }
                                    self.spine.set_total_ask(&pair, ask_sum);

                                    let bids = array[1].as_object().unwrap();
                                    let mut bid_sum: f64 = 0.0;
                                    for (price, size) in bids {
                                        let price: f64 = price.parse().unwrap();
                                        let size: f64 = size.as_string().unwrap().parse().unwrap();

                                        bid_sum += size * price;
                                    }
                                    bid_sum *= conversion_coef;
                                    self.spine.set_total_bid(&pair, bid_sum);

                                    let pair_tuple = self.spine.get_pairs().get(&pair).unwrap();
                                    info!(
                                        "new {:?} book on Poloniex with ask_sum: {}, bid_sum: {}",
                                        pair_tuple, ask_sum, bid_sum
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
                    } else if array[0].as_string().unwrap() == "t" {
                        // trades

                        self.parse_last_trade_info(pair.clone(), array.to_json().to_string());
                    }
                }
            }
        }
    }
}
