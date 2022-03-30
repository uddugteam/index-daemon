use crate::worker::market_helpers::market::{depth_helper_v2, Market};
use crate::worker::market_helpers::market_channels::ExternalMarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;
use reqwest::blocking::Client;

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

    fn get_channel_text_view(&self, channel: ExternalMarketChannels) -> String {
        match channel {
            ExternalMarketChannels::Ticker => {
                // Ticker channel of market FTX is not implemented, because it has no useful info.
                // Instead of websocket, we get needed info by REST API.
                panic!("Ticker channel of market FTX is not implemented, because it has no useful info.");
                // "ticker" 
            },
            ExternalMarketChannels::Trades => "trades",
            ExternalMarketChannels::Book => "orderbook",
        }
        .to_string()
    }

    fn get_websocket_url(&self, _pair: &str, _channel: ExternalMarketChannels) -> String {
        "wss://ftx.com/ws/".to_string()
    }

    fn get_websocket_on_open_msg(&self, pair: &str, channel: ExternalMarketChannels) -> Option<String> {
        Some(format!(
            "{{\"op\": \"subscribe\", \"channel\": \"{}\", \"market\": \"{}\"}}",
            self.get_channel_text_view(channel),
            pair,
        ))
    }

    fn update_ticker(&mut self, _pair: String) -> Option<()> {
        let response = Client::new().get("https://ftx.com/api/markets").send();

        let response = response.ok()?;
        let response = response.text().ok()?;
        let json: serde_json::Value = serde_json::from_str(&response).ok()?;

        let object = json.as_object()?;
        let array = object.get("result")?.as_array()?;

        for object in array {
            let object = object.as_object()?;

            let pair = object.get("name")?.as_str()?.to_string();
            // Process only needed pairs
            if self.spine.get_exchange_pairs().contains_key(&pair) {
                let volume = object.get("quoteVolume24h")?.as_f64()?;
                self.parse_ticker_json_inner(pair, volume);
            }
        }

        Some(())
    }

    fn parse_ticker_json(&mut self, _pair: String, _json: serde_json::Value) -> Option<()> {
        // Ticker channel of market FTX is not implemented, because it has no useful info.
        // Instead of websocket, we get needed info by REST API.
        panic!("Ticker channel of market FTX is not implemented, because it has no useful info.");
    }

    fn parse_last_trade_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;
        let array = object.get("data")?.as_array()?;

        for object in array {
            let object = object.as_object()?;

            let mut last_trade_volume: f64 = object.get("size")?.as_f64()?;
            let last_trade_price: f64 = object.get("price")?.as_f64()?;

            let trade_type = object.get("side")?.as_str()?;
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

    fn parse_depth_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;
        let object = object.get("data")?.as_object()?;

        if object.get("action")?.as_str()? == "partial" {
            let asks = object.get("asks")?;
            let bids = object.get("bids")?;

            let asks = depth_helper_v2(asks);
            let bids = depth_helper_v2(bids);
            self.parse_depth_json_inner(pair, asks, bids);
        }

        Some(())
    }
}
