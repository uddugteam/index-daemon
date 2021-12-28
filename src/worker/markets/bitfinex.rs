use rustc_serialize::json::{Array, Json};

use crate::worker::market_helpers::market::Market;
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Bitfinex {
    pub spine: MarketSpine,
}

impl Bitfinex {
    fn depth_helper(array: &Array) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
        let mut asks = Vec::new();
        let mut bids = Vec::new();

        for item in array {
            let item = item.as_array().unwrap();
            let price = item[0].as_f64().unwrap();
            let size = item[2].as_f64().unwrap();

            if size > 0.0 {
                // bid
                bids.push((price, size));
            } else {
                // ask
                asks.push((price, size));
            }
        }

        (asks, bids)
    }
}

impl Market for Bitfinex {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn make_pair(&self, pair: (&str, &str)) -> String {
        "t".to_string()
            + &(self.spine.get_masked_value(pair.0).to_string()
                + self.spine.get_masked_value(pair.1))
            .to_uppercase()
    }

    fn get_channel_text_view(&self, channel: MarketChannels) -> String {
        match channel {
            MarketChannels::Ticker => "ticker",
            MarketChannels::Trades => "trades",
            MarketChannels::Book => "book",
        }
        .to_string()
    }

    fn get_websocket_url(&self, _pair: &str, _channel: MarketChannels) -> String {
        "wss://api-pub.bitfinex.com/ws/2".to_string()
    }

    fn get_websocket_on_open_msg(&self, pair: &str, channel: MarketChannels) -> Option<String> {
        Some(format!(
            "{{\"event\":\"subscribe\", \"channel\":\"{}\", \"symbol\":\"{}\"}}",
            self.get_channel_text_view(channel),
            pair
        ))
    }

    /// Response description: https://docs.bitfinex.com/reference?ref=https://coder.social#rest-public-ticker
    fn parse_ticker_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(array) = json.as_array() {
                if array.len() >= 2 {
                    if let Some(array) = array[1].as_array() {
                        if array.len() >= 8 {
                            let volume: f64 = array[7].as_f64().unwrap();

                            self.parse_ticker_info_inner(pair, volume);
                        }
                    }
                }
            }
        }
    }

    /// Response description: https://docs.bitfinex.com/reference?ref=https://coder.social#rest-public-trades
    fn parse_last_trade_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(array) = json.as_array() {
                if array.len() >= 3 {
                    if let Some(array) = array[2].as_array() {
                        if array.len() >= 4 {
                            let last_trade_volume: f64 = array[2].as_f64().unwrap();
                            let last_trade_price: f64 = array[3].as_f64().unwrap();

                            self.parse_last_trade_info_inner(
                                pair,
                                last_trade_volume,
                                last_trade_price,
                            );
                        }
                    }
                }
            }
        }
    }

    /// Response description: https://docs.bitfinex.com/reference?ref=https://coder.social#rest-public-book
    fn parse_depth_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(array) = json.as_array() {
                if let Some(array) = array[1].as_array() {
                    if array[0].is_array() {
                        let (asks, bids) = Self::depth_helper(array);

                        self.parse_depth_info_inner(pair, asks, bids);
                    }
                }
            }
        }
    }
}
