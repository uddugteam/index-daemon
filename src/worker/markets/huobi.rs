use crate::worker::market_helpers::market::{depth_helper_v2, Market};
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Huobi {
    pub spine: MarketSpine,
}

impl Market for Huobi {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn get_channel_text_view(&self, channel: MarketChannels) -> String {
        match channel {
            MarketChannels::Ticker => "ticker",
            MarketChannels::Trades => "trade.detail",
            MarketChannels::Book => "depth.step0",
        }
        .to_string()
    }

    fn get_websocket_url(&self, _pair: &str, _channel: MarketChannels) -> String {
        "wss://api.huobi.pro/ws".to_string()
    }

    fn get_websocket_on_open_msg(&self, pair: &str, channel: MarketChannels) -> Option<String> {
        Some(format!(
            "{{\"sub\": \"market.{}.{}\"}}",
            pair,
            self.get_channel_text_view(channel)
        ))
    }

    fn parse_ticker_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?;
        let object = object.get("tick")?;
        let object = object.as_object()?;

        let volume: f64 = object.get("vol")?.as_f64()?;
        self.parse_ticker_json_inner(pair, volume);

        Some(())
    }

    fn parse_last_trade_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let object = json.as_object()?.get("tick")?;
        let array = object.as_object()?.get("data")?;

        for object in array.as_array()? {
            let object = object.as_object()?;

            let mut last_trade_volume: f64 = object.get("amount")?.as_f64()?;
            let last_trade_price: f64 = object.get("price")?.as_f64()?;

            let trade_type = object.get("direction")?.as_str()?;
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
        let object = json.as_object()?.get("tick")?;
        let object = object.as_object()?;
        let asks = object.get("asks")?;
        let bids = object.get("bids")?;

        let asks = depth_helper_v2(asks);
        let bids = depth_helper_v2(bids);
        self.parse_depth_json_inner(pair, asks, bids);

        Some(())
    }
}
