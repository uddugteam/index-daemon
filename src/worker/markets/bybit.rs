use crate::worker::market_helpers::market::{parse_str_from_json_object, Market};
use crate::worker::market_helpers::market_channels::ExternalMarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;
use async_trait::async_trait;

pub struct Bybit {
    pub spine: MarketSpine,
}

impl Bybit {
    fn depth_helper(array: &Vec<serde_json::Value>) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
        let mut asks = Vec::new();
        let mut bids = Vec::new();

        for item in array {
            let item = item.as_object().unwrap();
            let price: f64 = parse_str_from_json_object(item, "price").unwrap();
            let size = item.get("size").unwrap().as_f64().unwrap();

            let order_type = item.get("side").unwrap().as_str().unwrap();
            // TODO: Check whether inversion is right
            if order_type == "Sell" {
                // bid
                bids.push((price, size));
            } else if order_type == "Buy" {
                // ask
                asks.push((price, size));
            }
        }

        (asks, bids)
    }
}

#[async_trait]
impl Market for Bybit {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn get_channel_text_view(&self, channel: ExternalMarketChannels) -> String {
        match channel {
            ExternalMarketChannels::Ticker => "instrument_info.100ms",
            ExternalMarketChannels::Trades => "trade",
            ExternalMarketChannels::Book => "orderBook_200.100ms",
        }
        .to_string()
    }

    async fn get_websocket_url(&self, _pair: &str, _channel: ExternalMarketChannels) -> String {
        "wss://stream.bybit.com/realtime".to_string()
    }

    fn get_websocket_on_open_msg(
        &self,
        pair: &str,
        channel: ExternalMarketChannels,
    ) -> Option<String> {
        Some(format!(
            "{{\"op\":\"subscribe\",\"args\":[\"{}.{}\"]}}",
            self.get_channel_text_view(channel),
            pair,
        ))
    }

    async fn parse_ticker_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?.get("data")?.as_object()?;

        let volume = object.get("volume_24h")?.as_f64()?;
        self.parse_ticker_json_inner(pair, volume).await;

        Some(())
    }

    async fn parse_last_trade_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let array = json.as_object()?.get("data")?.as_array()?;

        for item in array {
            let item = item.as_object()?;

            let last_trade_price = item.get("price")?.as_f64()?;
            let mut last_trade_volume = item.get("size")?.as_f64()?;

            let trade_type = item.get("side")?.as_str()?;
            // TODO: Check whether inversion is right
            if trade_type == "Sell" {
                // sell
                last_trade_volume *= -1.0;
            } else if trade_type == "Buy" {
                // buy
            }

            self.parse_last_trade_json_inner(pair.clone(), last_trade_volume, last_trade_price)
                .await;
        }

        Some(())
    }

    async fn parse_depth_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let array = json.as_object()?.get("data")?.as_array()?;

        let (asks, bids) = Self::depth_helper(array);
        self.parse_depth_json_inner(pair, asks, bids);

        Some(())
    }
}
