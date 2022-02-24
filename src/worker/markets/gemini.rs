use reqwest::blocking::Client;

use crate::worker::market_helpers::market::{
    parse_str_from_json_array, parse_str_from_json_object, Market,
};
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Gemini {
    pub spine: MarketSpine,
}

impl Gemini {
    fn depth_helper(
        ask_sum: f64,
        bid_sum: f64,
        array: &Vec<serde_json::Value>,
    ) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
        let mut asks = vec![(1.0, ask_sum)];
        let mut bids = vec![(1.0, bid_sum)];

        for item in array {
            if let Some(item) = item.as_array() {
                let price: f64 = parse_str_from_json_array(item, 1).unwrap();
                let size: f64 = parse_str_from_json_array(item, 2).unwrap();

                let order_type = item[0].as_str().unwrap();
                // TODO: Check whether inversion is right
                if order_type == "sell" {
                    // bid
                    bids.push((price, size));
                } else if order_type == "buy" {
                    // ask
                    asks.push((price, size));
                }
            }
        }

        (asks, bids)
    }
}

impl Market for Gemini {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn get_channel_text_view(&self, _channel: MarketChannels) -> String {
        panic!("Market Gemini has no channels.");
    }

    fn get_websocket_url(&self, _pair: &str, _channel: MarketChannels) -> String {
        "wss://api.gemini.com/v2/marketdata".to_string()
    }

    fn get_websocket_on_open_msg(&self, pair: &str, _channel: MarketChannels) -> Option<String> {
        Some(format!("{{\"type\": \"subscribe\",\"subscriptions\":[{{\"name\":\"l2\",\"symbols\":[\"{}\"]}}]}}", pair))
    }

    fn update_ticker(&mut self, pair: String) -> Option<()> {
        let url = format!("https://api.gemini.com/v1/pubticker/{}", pair);
        let response = Client::new().get(url).send();

        let response = response.ok()?;
        let response = response.text().ok()?;

        let json: serde_json::Value = serde_json::from_str(&response).ok()?;
        let object = json.as_object()?;
        let object = object.get("volume")?.as_object()?;

        let pair_tuple = self.get_spine().get_pairs().get(&pair).unwrap();
        let volume = parse_str_from_json_object(object, &pair_tuple.1)?;
        self.parse_ticker_json_inner(pair, volume);

        Some(())
    }

    /// Market Gemini has no channels (i.e. has single general channel), so we parse channel data from its single channel
    fn parse_ticker_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;
        let message_type = object.get("type")?.as_str()?;

        if message_type == "trade" {
            // trades
            self.parse_last_trade_json(pair, json);
        } else if message_type == "l2_updates" {
            // book
            self.parse_depth_json(pair, json);
        } else {
            // no ticker
        }

        Some(())
    }

    fn parse_last_trade_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;
        let message_type = object.get("type")?.as_str()?;

        if message_type == "trade" {
            let last_trade_price: f64 = parse_str_from_json_object(object, "price")?;
            let mut last_trade_volume: f64 = parse_str_from_json_object(object, "quantity")?;

            let trade_type = object.get("side")?.as_str()?;
            // TODO: Check whether inversion is right
            if trade_type == "sell" {
                // sell
                last_trade_volume *= -1.0;
            } else if trade_type == "buy" {
                // buy
            }

            self.parse_last_trade_json_inner(pair, last_trade_volume, last_trade_price);
        }

        Some(())
    }

    fn parse_depth_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;
        let message_type = object.get("type")?.as_str()?;

        if message_type == "l2_updates" {
            let ask_sum: f64 = self.spine.get_exchange_pairs().get(&pair)?.get_total_ask();

            let bid_sum: f64 = self.spine.get_exchange_pairs().get(&pair)?.get_total_bid();

            let array = object.get("changes")?.as_array()?;
            let (asks, bids) = Self::depth_helper(ask_sum, bid_sum, array);
            self.parse_depth_json_inner(pair, asks, bids);
        }

        Some(())
    }
}
