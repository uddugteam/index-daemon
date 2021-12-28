use rustc_serialize::json::Json;

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

    fn make_pair(&self, pair: (&str, &str)) -> String {
        (self.spine.get_masked_value(pair.0).to_string()
            + "-"
            + self.spine.get_masked_value(pair.1))
        .to_uppercase()
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

    fn parse_ticker_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(object) = json.as_object() {
                // TODO: Check whether key `volume_24h` is right
                if let Some(volume) = parse_str_from_json_object::<f64>(object, "volume_24h") {
                    self.parse_ticker_info_inner(pair, volume);
                }
            }
        }
    }

    fn parse_last_trade_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(object) = json.as_object() {
                if let Some(last_trade_volume) = parse_str_from_json_object(object, "size") {
                    if let Some(last_trade_price) =
                        parse_str_from_json_object::<f64>(object, "price")
                    {
                        self.parse_last_trade_info_inner(pair, last_trade_volume, last_trade_price);
                    }
                }
            }
        }
    }

    fn parse_depth_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(object) = json.as_object() {
                if let Some(asks) = object.get("asks") {
                    if let Some(bids) = object.get("bids") {
                        let asks = depth_helper_v1(asks);
                        let bids = depth_helper_v1(bids);

                        self.parse_depth_info_inner(pair, asks, bids);
                    }
                }
            }
        }
    }
}
