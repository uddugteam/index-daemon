use rustc_serialize::json::{Array, Json};

use crate::worker::market_helpers::market::{
    depth_helper_v1, parse_str_from_json_array, parse_str_from_json_object, Market,
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
        array: &Array,
    ) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
        let mut asks = vec![(1.0, ask_sum)];
        let mut bids = vec![(1.0, bid_sum)];

        for item in array {
            if let Some(item) = item.as_array() {
                let price: f64 = parse_str_from_json_array(item, 1).unwrap();
                let size: f64 = parse_str_from_json_array(item, 2).unwrap();

                let order_type = item[0].as_string().unwrap();
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

    fn make_pair(&self, pair: (&str, &str)) -> String {
        (self.spine.get_masked_value(pair.0).to_string() + self.spine.get_masked_value(pair.1))
            .to_uppercase()
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

    /// Market Gemini has no channels (i.e. has single general channel), so we parse channel data from its single channel
    fn parse_ticker_json(&mut self, pair: String, json: Json) -> Option<()> {
        let object = json.as_object()?;
        let message_type = object.get("type")?.as_string()?;

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

    fn parse_last_trade_json(&mut self, pair: String, json: Json) -> Option<()> {
        let object = json.as_object()?;
        let message_type = object.get("type")?.as_string()?;

        if message_type == "trade" {
            let mut last_trade_price: f64 = parse_str_from_json_object(object, "price")?;
            let last_trade_volume: f64 = parse_str_from_json_object(object, "quantity")?;

            let trade_type = object.get("side")?.as_string()?;
            // TODO: Check whether inversion is right
            if trade_type == "sell" {
                // sell
                last_trade_price *= -1.0;
            } else if trade_type == "buy" {
                // buy
            }

            self.parse_last_trade_json_inner(pair, last_trade_volume, last_trade_price);
        }

        Some(())
    }

    fn parse_depth_json(&mut self, pair: String, json: Json) -> Option<()> {
        let object = json.as_object()?;
        let message_type = object.get("type")?.as_string()?;

        if message_type == "l2_updates" {
            let ask_sum: f64 = self
                .spine
                .get_exchange_pairs()
                .get(&pair)
                .unwrap()
                .get_total_ask();

            let bid_sum: f64 = self
                .spine
                .get_exchange_pairs()
                .get(&pair)
                .unwrap()
                .get_total_bid();

            let array = object.get("changes")?.as_array()?;
            let (asks, bids) = Self::depth_helper(ask_sum, bid_sum, array);
            self.parse_depth_json_inner(pair, asks, bids);
        }

        Some(())
    }
}
