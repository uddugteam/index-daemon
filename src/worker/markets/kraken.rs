use rustc_serialize::json::Json;

use crate::worker::market_helpers::market::{depth_helper_v1, parse_str_from_json_array, Market};
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Kraken {
    pub spine: MarketSpine,
}

impl Market for Kraken {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn make_pair(&self, pair: (&str, &str)) -> String {
        (self.spine.get_masked_value(pair.0).to_string()
            + "/"
            + self.spine.get_masked_value(pair.1))
        .to_uppercase()
    }

    fn get_channel_text_view(&self, channel: MarketChannels) -> String {
        match channel {
            MarketChannels::Ticker => "ticker",
            MarketChannels::Trades => "trade",
            MarketChannels::Book => "book",
        }
        .to_string()
    }

    fn get_websocket_url(&self, _pair: &str, _channel: MarketChannels) -> String {
        "wss://ws.kraken.com".to_string()
    }

    fn get_websocket_on_open_msg(&self, pair: &str, channel: MarketChannels) -> Option<String> {
        Some(format!(
            "{{\"event\": \"subscribe\", \"pair\": [\"{}\"], \"subscription\": {{\"name\": \"{}\"}}}}",
            pair, self.get_channel_text_view(channel)
        ))
    }

    fn parse_ticker_json(&mut self, pair: String, json: Json) -> Option<()> {
        let array = json.as_array()?;
        let array = array[1].as_object()?;

        let base_price = array.get("c")?.as_array()?;
        let base_price: f64 = parse_str_from_json_array(base_price, 0)?;

        let base_volume = array.get("v")?.as_array()?;
        let base_volume: f64 = parse_str_from_json_array(base_volume, 1)?;

        let quote_volume: f64 = base_volume * base_price;

        self.parse_ticker_json_inner(pair, quote_volume);

        Some(())
    }

    fn parse_last_trade_json(&mut self, pair: String, json: Json) -> Option<()> {
        let array = json.as_array()?;

        for array in array[1].as_array()? {
            let array = array.as_array()?;

            let last_trade_price: f64 = parse_str_from_json_array(array, 0)?;
            let mut last_trade_volume: f64 = parse_str_from_json_array(array, 1)?;

            let trade_type = array[3].as_string()?;
            // TODO: Check whether inversion is right
            if trade_type == "s" {
                // sell
                last_trade_volume *= -1.0;
            } else if trade_type == "b" {
                // buy
            }

            self.parse_last_trade_json_inner(pair.clone(), last_trade_volume, last_trade_price);
        }

        Some(())
    }

    fn parse_depth_json(&mut self, pair: String, json: Json) -> Option<()> {
        let array = json.as_array()?;
        let object = array[1].as_object()?;
        let asks = object.get("as")?;
        let bids = object.get("bs")?;

        let asks = depth_helper_v1(asks);
        let bids = depth_helper_v1(bids);
        self.parse_depth_json_inner(pair, asks, bids);

        Some(())
    }
}
