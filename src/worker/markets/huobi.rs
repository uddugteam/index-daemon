use rustc_serialize::json::Json;

use crate::worker::market_helpers::market::Market;
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Huobi {
    pub spine: MarketSpine,
}

impl Huobi {
    fn depth_helper(json: &Json) -> Vec<(f64, f64)> {
        json.as_array()
            .unwrap()
            .iter()
            .map(|v| {
                let v = v.as_array().unwrap();
                (v[0].as_f64().unwrap(), v[1].as_f64().unwrap())
            })
            .collect()
    }
}

impl Market for Huobi {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn make_pair(&self, pair: (&str, &str)) -> String {
        (self.spine.get_masked_value(pair.0).to_string() + self.spine.get_masked_value(pair.1))
            .to_lowercase()
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

    fn parse_ticker_json(&mut self, pair: String, json: Json) -> Option<()> {
        let object = json.as_object()?;
        let object = object.get("tick")?;
        let object = object.as_object()?;

        let volume: f64 = object.get("vol").unwrap().as_f64().unwrap();
        self.parse_ticker_json_inner(pair, volume);

        Some(())
    }

    fn parse_last_trade_json(&mut self, pair: String, json: Json) -> Option<()> {
        let object = json.as_object().unwrap().get("tick")?;
        let array = object.as_object().unwrap().get("data")?;

        for object in array.as_array().unwrap() {
            let object = object.as_object().unwrap();

            let last_trade_volume: f64 = object.get("amount").unwrap().as_f64().unwrap();
            let mut last_trade_price: f64 = object.get("price").unwrap().as_f64().unwrap();

            let trade_type = object.get("direction").unwrap().as_string().unwrap();
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
        let object = json.as_object().unwrap().get("tick")?;
        let object = object.as_object()?;
        let asks = object.get("asks")?;
        let bids = object.get("bids")?;

        let asks = Self::depth_helper(asks);
        let bids = Self::depth_helper(bids);
        self.parse_depth_json_inner(pair, asks, bids);

        Some(())
    }
}
