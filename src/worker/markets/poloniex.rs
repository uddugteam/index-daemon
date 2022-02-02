use chrono::{DateTime, Utc, MIN_DATETIME};
use reqwest::blocking::Client;
use std::collections::HashMap;

use crate::worker::defaults::POLONIEX_EXCHANGE_PAIRS;
use crate::worker::market_helpers::market::{
    parse_str_from_json_array, parse_str_from_json_object, Market,
};
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Poloniex {
    pub spine: MarketSpine,
    pair_codes: HashMap<(String, String), String>,
    exchange_pairs_last_price: HashMap<String, f64>,
    last_http_request_timestamp: DateTime<Utc>,
}

impl Poloniex {
    pub fn new(spine: MarketSpine) -> Self {
        let mut pair_codes = HashMap::new();
        for (pair_code, pair_tuple) in POLONIEX_EXCHANGE_PAIRS {
            let pair: (String, String) = (
                spine.get_unmasked_value(pair_tuple.0).to_string(),
                spine.get_unmasked_value(pair_tuple.1).to_string(),
            );
            let pair_reversed = (pair.1.to_string(), pair.0.to_string());

            pair_codes.insert(pair, pair_code.to_string());
            pair_codes.insert(pair_reversed, pair_code.to_string());
        }

        Self {
            spine,
            pair_codes,
            exchange_pairs_last_price: HashMap::new(),
            last_http_request_timestamp: MIN_DATETIME,
        }
    }

    fn depth_helper(json: &serde_json::Value) -> Vec<(f64, f64)> {
        json.as_object()
            .unwrap()
            .iter()
            .map(|(price, size)| {
                (
                    price.parse().unwrap(),
                    size.as_str().unwrap().parse().unwrap(),
                )
            })
            .collect()
    }

    fn parse_pair_code_from_pair_string(&self, pair_string: &str) -> Option<String> {
        let pair_tuple: Vec<&str> = pair_string.split('_').collect();

        let pair_tuple = (pair_tuple.get(0)?, pair_tuple.get(1)?);

        let pair_tuple = (
            self.spine.get_unmasked_value(pair_tuple.0).to_string(),
            self.spine.get_unmasked_value(pair_tuple.1).to_string(),
        );

        self.pair_codes.get(&pair_tuple).cloned()
    }

    fn coin_exists(&self, coin: &str) -> bool {
        let pair = (coin.to_string(), "USD".to_string());
        self.pair_codes.contains_key(&pair)
    }

    fn refresh_pair_prices(&mut self) -> Option<()> {
        // Hold 10 seconds between HTTP requests to Poloniex
        if (Utc::now() - self.last_http_request_timestamp).num_milliseconds() > 10000 {
            self.last_http_request_timestamp = Utc::now();

            let response = Client::new()
                .post("https://poloniex.com/public?command=returnTicker")
                .send();

            let response = response.ok()?;
            let response = response.text().ok()?;
            let json: serde_json::Value = serde_json::from_str(&response).ok()?;

            let object = json.as_object()?;
            for (pair_string, object) in object {
                let object = object.as_object()?;

                if let Some(pair_code) = self.parse_pair_code_from_pair_string(pair_string) {
                    let price: f64 = parse_str_from_json_object(object, "last")?;

                    self.exchange_pairs_last_price.insert(pair_code, price);
                }
            }
        }

        Some(())
    }

    fn get_pair_price(&mut self, pair_code: &str) -> Option<f64> {
        self.refresh_pair_prices();

        self.exchange_pairs_last_price.get(pair_code).cloned()
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
    fn parse_ticker_json(&mut self, _pair: String, json: serde_json::Value) -> Option<()> {
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
                    v.as_str().unwrap().parse().unwrap(),
                )
            })
            .filter(|(k, _)| {
                // Remove unneeded pairs
                self.spine.get_exchange_pairs().contains_key(k)
            })
            .collect();

        for (pair_code, base_volume) in volumes {
            if let Some(base_price) = self.get_pair_price(&pair_code) {
                let quote_volume: f64 = base_volume * base_price;

                self.parse_ticker_json_inner(pair_code, quote_volume);
            }
        }

        Some(())
    }

    fn parse_last_trade_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
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

    fn parse_depth_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let json = json.as_array()?.get(2)?;

        for array in json.as_array()? {
            let array = array.as_array()?;

            if array[0].as_str()? == "i" {
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
            } else if array[0].as_str()? == "t" {
                // trades

                self.parse_last_trade_json(pair.clone(), serde_json::Value::from(array.as_slice()));
            }
        }

        Some(())
    }
}
