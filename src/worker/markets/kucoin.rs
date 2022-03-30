use reqwest::blocking::Client;
use std::thread;
use std::time;

use crate::worker::market_helpers::market::{depth_helper_v1, Market};
use crate::worker::market_helpers::market_channels::ExternalMarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Kucoin {
    pub spine: MarketSpine,
}

impl Kucoin {
    fn get_token() -> String {
        let inner_fn = || {
            let response = Client::new()
                .post("https://api.kucoin.com/api/v1/bullet-public")
                .send();

            let response = response.ok()?;
            let response = response.text().ok()?;
            let json: serde_json::Value = serde_json::from_str(&response).ok()?;

            let object = json.as_object()?;
            let object = object.get("data")?;
            let object = object.as_object()?;

            let token = object.get("token")?;
            let token = token.as_str()?;

            Some(token.to_string())
        };

        loop {
            if let Some(token) = inner_fn() {
                return token;
            } else {
                error!("api.kucoin.com: Get token error.");
                thread::sleep(time::Duration::from_millis(10000));
            }
        }
    }
}

impl Market for Kucoin {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn get_channel_text_view(&self, channel: ExternalMarketChannels) -> String {
        match channel {
            ExternalMarketChannels::Ticker => "market/snapshot",
            ExternalMarketChannels::Trades => {
                // TODO: Implement
                // Implementation is too hard and requires Private API key
                panic!("Trades channel for Kucoin is not implemented.");
            }
            ExternalMarketChannels::Book => "spotMarket/level2Depth50",
        }
        .to_string()
    }

    fn get_websocket_url(&self, _pair: &str, _channel: ExternalMarketChannels) -> String {
        let token = Self::get_token();

        format!("wss://ws-api.kucoin.com/endpoint?token={}", token)
    }

    fn get_websocket_on_open_msg(&self, pair: &str, channel: ExternalMarketChannels) -> Option<String> {
        Some(format!(
            "{{\"type\": \"subscribe\", \"topic\": \"/{}:{}\"}}",
            self.get_channel_text_view(channel),
            pair,
        ))
    }

    fn parse_ticker_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;
        let object = object.get("data")?.as_object()?;
        let object = object.get("data")?.as_object()?;

        let volume: f64 = object.get("volValue")?.as_f64()?;
        self.parse_ticker_json_inner(pair, volume);

        Some(())
    }

    /// TODO: Implement
    fn parse_last_trade_json(&mut self, _pair: String, _json: serde_json::Value) -> Option<()> {
        panic!("Trades channel for Kucoin is not implemented.");
    }

    fn parse_depth_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;
        let object = object.get("data")?.as_object()?;
        let asks = object.get("asks")?;
        let bids = object.get("bids")?;

        let asks = depth_helper_v1(asks);
        let bids = depth_helper_v1(bids);
        self.parse_depth_json_inner(pair, asks, bids);

        Some(())
    }
}
