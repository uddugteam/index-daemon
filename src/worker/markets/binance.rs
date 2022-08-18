use crate::worker::market_helpers::market::{depth_helper_v1, parse_str_from_json_object, Market};
use crate::worker::market_helpers::market_channels::ExternalMarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;
use async_trait::async_trait;

pub struct Binance {
    pub spine: MarketSpine,
}

#[async_trait]
impl Market for Binance {
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
            ExternalMarketChannels::Book => "depth20",
        }
        .to_string()
    }

    async fn get_websocket_url(&self, pair: &str, channel: ExternalMarketChannels) -> String {
        format!(
            "wss://stream.binance.com:9443/ws/{}@{}",
            pair,
            self.get_channel_text_view(channel)
        )
    }

    fn get_websocket_on_open_msg(
        &self,
        _pair: &str,
        _channel: ExternalMarketChannels,
    ) -> Option<String> {
        None
    }

    async fn parse_ticker_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;

        let volume: f64 = parse_str_from_json_object(object, "q")?;
        self.parse_ticker_json_inner(&pair, volume).await;

        Some(())
    }

    async fn parse_last_trade_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;

        let last_trade_volume: f64 = parse_str_from_json_object(object, "q")?;
        let last_trade_price: f64 = parse_str_from_json_object(object, "p")?;
        self.parse_last_trade_json_inner(&pair, last_trade_volume, last_trade_price)
            .await;

        Some(())
    }

    async fn parse_depth_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;
        let asks = object.get("asks")?;
        let bids = object.get("bids")?;

        let asks = depth_helper_v1(asks);
        let bids = depth_helper_v1(bids);
        self.parse_depth_json_inner(&pair, asks, bids);

        Some(())
    }
}
