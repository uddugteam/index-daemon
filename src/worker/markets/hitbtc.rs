use rustc_serialize::json::Json;

use crate::worker::market_helpers::market::{depth_helper_v1, parse_str_from_json_object, Market};
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

    fn parse_ticker_json(&mut self, pair: String, json: Json) -> Option<()> {
        let object = json.as_object()?;
        let object = object.get("data")?;
        let object = object.as_object()?.get(&pair)?.as_object()?;

        let volume: f64 = parse_str_from_json_object(object, "q")?;
        self.parse_ticker_json_inner(pair, volume);

        Some(())
    }

    fn parse_last_trade_json(&mut self, pair: String, json: Json) -> Option<()> {
        let object = json.as_object()?;
        let object = object.get("update")?;
        let array = object.as_object()?.get(&pair)?.as_array()?;

        for object in array {
            let object = object.as_object()?;

            let mut last_trade_volume: f64 = parse_str_from_json_object(object, "q")?;
            let last_trade_price: f64 = parse_str_from_json_object(object, "p")?;

            let trade_type = object.get("s")?.as_string()?;
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
        let object = object.get("snapshot")?;
        let object = object.as_object()?.get(&pair)?.as_object()?;
        let asks = object.get("a")?;
        let bids = object.get("b")?;

        let asks = depth_helper_v1(asks);
        let bids = depth_helper_v1(bids);
        self.parse_depth_json_inner(pair, asks, bids);

        Some(())
    }
}
