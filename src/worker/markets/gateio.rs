use rand::Rng;
use rustc_serialize::json::Json;

use crate::worker::market_helpers::market::{depth_helper_v1, parse_str_from_json_object, Market};
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Gateio {
    pub spine: MarketSpine,
}

impl Market for Gateio {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn make_pair(&self, pair: (&str, &str)) -> String {
        (self.spine.get_masked_value(pair.0).to_string()
            + "_"
            + self.spine.get_masked_value(pair.1))
        .to_uppercase()
    }

    fn get_channel_text_view(&self, channel: MarketChannels) -> String {
        match channel {
            MarketChannels::Ticker => "ticker.subscribe",
            MarketChannels::Trades => "trades.subscribe",
            MarketChannels::Book => "depth.subscribe",
        }
        .to_string()
    }

    fn get_websocket_url(&self, _pair: &str, _channel: MarketChannels) -> String {
        "wss://ws.gate.io/v3/".to_string()
    }

    fn get_websocket_on_open_msg(&self, pair: &str, channel: MarketChannels) -> Option<String> {
        let id = rand::thread_rng().gen_range(10000..10000000);
        let params = if let MarketChannels::Book = channel {
            ", 5, \"0.0001\""
        } else {
            ""
        };

        Some(format!(
            "{{\"id\":{}, \"method\":\"{}\", \"params\":[\"{}\"{}]}}",
            id,
            self.get_channel_text_view(channel),
            pair,
            params
        ))
    }

    fn parse_ticker_json(&mut self, pair: String, json: Json) -> Option<()> {
        let object = json.as_object()?;
        let object = object.get("params")?.as_array()?.get(1)?.as_object()?;

        let volume: f64 = parse_str_from_json_object(object, "baseVolume")?;
        self.parse_ticker_json_inner(pair, volume);

        Some(())
    }

    fn parse_last_trade_json(&mut self, pair: String, json: Json) -> Option<()> {
        let object = json.as_object()?;
        let array = object.get("params")?.as_array()?.get(1)?.as_array()?;

        for object in array {
            let object = object.as_object()?;

            let mut last_trade_volume: f64 = parse_str_from_json_object(object, "amount")?;
            let last_trade_price: f64 = parse_str_from_json_object(object, "price")?;

            let trade_type = object.get("type")?.as_string()?;
            // TODO: Check whether inversion is right
            if trade_type == "sell" {
                // sell
                last_trade_volume *= -1.0;
            } else if trade_type == "buy" {
                // buy
            }

            self.parse_last_trade_json_inner(pair.clone(), last_trade_volume, last_trade_price);
        }

        Some(())
    }

    fn parse_depth_json(&mut self, pair: String, json: Json) -> Option<()> {
        let object = json.as_object()?;
        let object = object.get("params")?.as_array()?.get(1)?.as_object()?;
        let asks = object.get("asks")?;
        let bids = object.get("bids")?;

        let asks = depth_helper_v1(asks);
        let bids = depth_helper_v1(bids);
        self.parse_depth_json_inner(pair, asks, bids);

        Some(())
    }
}
