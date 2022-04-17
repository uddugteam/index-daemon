use crate::worker::market_helpers::market::{depth_helper_v1, parse_str_from_json_object, Market};
use crate::worker::market_helpers::market_channels::ExternalMarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;
use async_trait::async_trait;

pub struct Okcoin {
    pub spine: MarketSpine,
}

#[async_trait]
impl Market for Okcoin {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn get_channel_text_view(&self, channel: ExternalMarketChannels) -> String {
        match channel {
            ExternalMarketChannels::Ticker => "ticker",
            ExternalMarketChannels::Trades => "trade",
            ExternalMarketChannels::Book => "depth_l2_tbt",
            // MarketChannels::Book => "depth",
        }
        .to_string()
    }

    async fn get_websocket_url(&self, _pair: &str, _channel: ExternalMarketChannels) -> String {
        "wss://real.okcoin.com:8443/ws/v3".to_string()
    }

    fn get_websocket_on_open_msg(
        &self,
        pair: &str,
        channel: ExternalMarketChannels,
    ) -> Option<String> {
        Some(format!(
            "{{\"op\": \"subscribe\", \"args\": [\"spot/{}:{}\"]}}",
            self.get_channel_text_view(channel),
            pair
        ))
    }

    async fn parse_ticker_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let array = json.as_object()?.get("data")?;

        for object in array.as_array()? {
            let object = object.as_object()?;
            let volume: f64 = parse_str_from_json_object(object, "quote_volume_24h")?;

            self.parse_ticker_json_inner(pair.clone(), volume).await;
        }

        Some(())
    }

    async fn parse_last_trade_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let array = json.as_object()?.get("data")?;

        for object in array.as_array()? {
            let object = object.as_object()?;

            let mut last_trade_volume: f64 = parse_str_from_json_object(object, "size")?;
            let last_trade_price: f64 = parse_str_from_json_object(object, "price")?;

            let trade_type = object.get("side")?.as_str()?;
            // TODO: Check whether inversion is right
            if trade_type == "sell" {
                // sell
                last_trade_volume *= -1.0;
            } else if trade_type == "buy" {
                // buy
            }

            self.parse_last_trade_json_inner(pair.clone(), last_trade_volume, last_trade_price)
                .await;
        }

        Some(())
    }

    async fn parse_depth_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;
        let array = object.get("data")?;

        if object.get("action")?.as_str()? == "partial" {
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
