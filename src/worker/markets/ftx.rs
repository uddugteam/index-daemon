use rustc_serialize::json::Json;

use crate::worker::market_helpers::market::{
    depth_helper_v1, depth_helper_v2, parse_str_from_json_object, Market,
};
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Ftx {
    pub spine: MarketSpine,
}

impl Market for Ftx {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn get_channel_text_view(&self, channel: MarketChannels) -> String {
        match channel {
            MarketChannels::Ticker => "ticker",
            MarketChannels::Trades => "trades",
            MarketChannels::Book => "orderbook",
        }
        .to_string()
    }

    fn get_websocket_url(&self, _pair: &str, _channel: MarketChannels) -> String {
        "wss://ftx.com/ws/".to_string()
    }

    fn get_websocket_on_open_msg(&self, pair: &str, channel: MarketChannels) -> Option<String> {
        Some(format!(
            "{{\"op\": \"subscribe\", \"channel\": \"{}\", \"market\": \"{}\"}}",
            self.get_channel_text_view(channel),
            pair,
        ))
    }

    fn parse_ticker_json(&mut self, pair: String, json: Json) -> Option<()> {
        let object = json.as_object()?;
        let object = object.get("data")?.as_object()?;

        // TODO: Check whether chosen keys and calculation method are right
        let ask_size = object.get("askSize")?.as_f64()?;
        let bid_size = object.get("bidSize")?.as_f64()?;
        let volume = ask_size + bid_size;
        self.parse_ticker_json_inner(pair, volume);

        Some(())
    }

    fn parse_last_trade_json(&mut self, pair: String, json: Json) -> Option<()> {
        let object = json.as_object()?;
        let array = object.get("data")?.as_array()?;

        for object in array {
            let object = object.as_object()?;

            let mut last_trade_volume: f64 = object.get("size")?.as_f64()?;
            let last_trade_price: f64 = object.get("price")?.as_f64()?;

            let trade_type = object.get("side")?.as_string()?;
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
        let object = object.get("data")?.as_object()?;

        if object.get("action")?.as_string()? == "partial" {
            let asks = object.get("asks")?;
            let bids = object.get("bids")?;

            let asks = depth_helper_v2(asks);
            let bids = depth_helper_v2(bids);
            self.parse_depth_json_inner(pair, asks, bids);
        }

        Some(())
    }
}
