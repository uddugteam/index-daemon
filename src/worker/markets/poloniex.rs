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
    fn depth_helper(json: &Json) -> Vec<(f64, f64)> {
        json.as_object()
            .unwrap()
            .iter()
            .map(|(price, size)| {
                (
                    price.parse().unwrap(),
                    size.as_string().unwrap().parse().unwrap(),
                )
            })
            .collect()
    }
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
            MarketChannels::Ticker => "1003",
            MarketChannels::Trades => {
                // There are no distinct Trades channel in Poloniex. We get Trades inside of Book channel.
                panic!("Poloniex: Subscription to wrong channel: Trades.")
            }
            MarketChannels::Book => {
                // This string was intentionally left blank, because Poloniex don't have code for Book
                // and we pass pair code instead of it (we do this in fn get_websocket_on_open_msg)
                ""
            }
        }
        .to_string()
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

    /// Poloniex sends us coin instead of pair, then we create pair coin-USD
    /// TODO: Check whether function takes right values from json (in the meaning of coin/pair misunderstanding)
    fn parse_ticker_json(&mut self, _pair: String, json: Json) -> Option<()> {
        let array = json.as_array()?.get(2)?;
        let object = array.as_array()?.get(2)?.as_object()?;

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
                self.spine.get_exchange_pairs().contains_key(k)
            })
            .collect();

        for (pair_code, volume) in volumes {
            self.parse_ticker_json_inner(pair_code, volume);
        }

        Some(())
    }

    fn parse_last_trade_json(&mut self, pair: String, json: Json) -> Option<()> {
        let array = json.as_array()?;

        let last_trade_price: f64 = parse_str_from_json_array(array, 3)?;
        let mut last_trade_volume: f64 = parse_str_from_json_array(array, 4)?;

        let trade_type = array[2].as_u64()?;
        // TODO: Check whether inversion is right
        if trade_type == 0 {
            // sell
            last_trade_volume *= -1.0;
        } else if trade_type == 1 {
            // buy
        }

        self.parse_last_trade_json_inner(pair, last_trade_volume, last_trade_price);

        Some(())
    }

    fn parse_depth_json(&mut self, pair: String, json: Json) -> Option<()> {
        let json = json.as_array()?.get(2)?;

        for array in json.as_array()? {
            let array = array.as_array()?;

            if array[0].as_string()? == "i" {
                // book
                if let Some(object) = array.get(1)?.as_object() {
                    if let Some(object) = object.get("orderBook") {
                        if let Some(array) = object.as_array() {
                            let asks = &array[0];
                            let bids = &array[1];

                            let asks = Self::depth_helper(asks);
                            let bids = Self::depth_helper(bids);

                            self.parse_depth_json_inner(pair.clone(), asks, bids);
                        }
                    }
                }
            } else if array[0].as_string()? == "t" {
                // trades

                self.parse_last_trade_json(pair.clone(), array.to_json());
            }
        }

        Some(())
    }
}
