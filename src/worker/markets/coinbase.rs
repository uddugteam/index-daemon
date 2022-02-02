use crate::worker::market_helpers::market::{depth_helper_v1, parse_str_from_json_object, Market};
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Coinbase {
    pub spine: MarketSpine,
}

impl Market for Coinbase {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn get_channel_text_view(&self, channel: MarketChannels) -> String {
        match channel {
            MarketChannels::Ticker => "ticker",
            MarketChannels::Trades => "matches",
            MarketChannels::Book => "level2",
            // MarketChannels::Book => "full",
        }
        .to_string()
    }

    fn get_websocket_url(&self, _pair: &str, _channel: MarketChannels) -> String {
        "wss://ws-feed.pro.coinbase.com".to_string()
    }

    fn get_websocket_on_open_msg(&self, pair: &str, channel: MarketChannels) -> Option<String> {
        Some(format!(
            "{{\"type\": \"subscribe\", \"product_ids\": [\"{}\"], \"channels\": [\"{}\"]}}",
            pair,
            self.get_channel_text_view(channel)
        ))
    }

    fn parse_ticker_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;

        let base_price: f64 = parse_str_from_json_object(object, "price")?;
        let base_volume: f64 = parse_str_from_json_object(object, "volume_24h")?;
        let quote_volume: f64 = base_volume * base_price;

        self.parse_ticker_json_inner(pair, quote_volume);

        Some(())
    }

    fn parse_last_trade_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;

        let last_trade_volume: f64 = parse_str_from_json_object(object, "size")?;
        let last_trade_price: f64 = parse_str_from_json_object(object, "price")?;
        self.parse_last_trade_json_inner(pair, last_trade_volume, last_trade_price);

        Some(())
    }

    fn parse_depth_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;
        let asks = object.get("asks")?;
        let bids = object.get("bids")?;

        let asks = depth_helper_v1(asks);
        let bids = depth_helper_v1(bids);
        self.parse_depth_json_inner(pair, asks, bids);

        Some(())
    }
}
