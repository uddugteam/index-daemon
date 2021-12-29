use rustc_serialize::json::Json;

use crate::worker::market_helpers::market::{depth_helper_v1, parse_str_from_json_object, Market};
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Okcoin {
    pub spine: MarketSpine,
}

impl Market for Okcoin {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn make_pair(&self, pair: (&str, &str)) -> String {
        (self.spine.get_masked_value(pair.0).to_string()
            + "-"
            + self.spine.get_masked_value(pair.1))
        .to_uppercase()
    }

    fn get_channel_text_view(&self, channel: MarketChannels) -> String {
        match channel {
            MarketChannels::Ticker => "ticker",
            MarketChannels::Trades => "trade",
            MarketChannels::Book => "depth_l2_tbt",
            // MarketChannels::Book => "depth",
        }
        .to_string()
    }

    fn get_websocket_url(&self, _pair: &str, _channel: MarketChannels) -> String {
        "wss://real.okcoin.com:8443/ws/v3".to_string()
    }

    fn get_websocket_on_open_msg(&self, pair: &str, channel: MarketChannels) -> Option<String> {
        Some(format!(
            "{{\"op\": \"subscribe\", \"args\": [\"spot/{}:{}\"]}}",
            self.get_channel_text_view(channel),
            pair
        ))
    }

    fn parse_ticker_json(&mut self, pair: String, json: Json) -> Option<()> {
        let array = json.as_object()?.get("data")?;

        for object in array.as_array()? {
            let object = object.as_object()?;

            // TODO: Check whether key `base_volume_24h` is right
            let volume: f64 = parse_str_from_json_object(object, "base_volume_24h")?;

            self.parse_ticker_json_inner(pair.clone(), volume);
        }

        Some(())
    }

    fn parse_last_trade_json(&mut self, pair: String, json: Json) -> Option<()> {
        let array = json.as_object()?.get("data")?;

        for object in array.as_array()? {
            let object = object.as_object()?;

            let last_trade_volume: f64 = parse_str_from_json_object(object, "size")?;
            let mut last_trade_price: f64 = parse_str_from_json_object(object, "price")?;

            let trade_type = object.get("side")?.as_string()?;
            // TODO: Check whether inversion is right
            if trade_type == "sell" {
                // sell
                last_trade_price *= -1.0;
            } else if trade_type == "buy" {
                // buy
            }

            self.parse_last_trade_json_inner(pair.clone(), last_trade_volume, last_trade_price);
        }

        Some(())
    }

    fn parse_depth_json(&mut self, pair: String, json: Json) -> Option<()> {
        let object = json.as_object()?;
        let array = object.get("data")?;

        if object.get("action")?.as_string()? == "partial" {
            for object in array.as_array()? {
                let object = object.as_object()?;

                if let Some(asks) = object.get("asks") {
                    if let Some(bids) = object.get("bids") {
                        let asks = depth_helper_v1(asks);
                        let bids = depth_helper_v1(bids);

                        self.parse_depth_json_inner(pair.clone(), asks, bids);
                    }
                }
            }
        }

        Some(())
    }
}
