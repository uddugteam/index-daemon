use crate::worker::market_helpers::market::Market;
use crate::worker::market_helpers::market_channels::ExternalMarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Bitfinex {
    pub spine: MarketSpine,
}

impl Bitfinex {
    fn depth_helper(array: &Vec<serde_json::Value>) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
        let mut asks = Vec::new();
        let mut bids = Vec::new();

        for item in array {
            if let Some(item) = item.as_array() {
                let price = item[0].as_f64().unwrap();
                let size = item[2].as_f64().unwrap();

                if size > 0.0 {
                    // bid
                    bids.push((price, size));
                } else {
                    // ask
                    asks.push((price, size));
                }
            }
        }

        (asks, bids)
    }
}

impl Market for Bitfinex {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn make_pair(&self, pair: (&str, &str)) -> String {
        "t".to_string()
            + &(self.spine.get_masked_value(pair.0).to_string()
                + self.spine.get_masked_value(pair.1))
            .to_uppercase()
    }

    fn get_channel_text_view(&self, channel: ExternalMarketChannels) -> String {
        match channel {
            ExternalMarketChannels::Ticker => "ticker",
            ExternalMarketChannels::Trades => "trades",
            ExternalMarketChannels::Book => "book",
        }
        .to_string()
    }

    fn get_websocket_url(&self, _pair: &str, _channel: ExternalMarketChannels) -> String {
        "wss://api-pub.bitfinex.com/ws/2".to_string()
    }

    fn get_websocket_on_open_msg(&self, pair: &str, channel: ExternalMarketChannels) -> Option<String> {
        Some(format!(
            "{{\"event\":\"subscribe\", \"channel\":\"{}\", \"symbol\":\"{}\"}}",
            self.get_channel_text_view(channel),
            pair
        ))
    }

    /// Response description: https://docs.bitfinex.com/reference?ref=https://coder.social#rest-public-ticker
    fn parse_ticker_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let array = json.as_array()?;
        let array = array[1].as_array()?;

        let base_price: f64 = array[6].as_f64()?;
        let base_volume: f64 = array[7].as_f64()?;
        let quote_volume: f64 = base_volume * base_price;

        self.parse_ticker_json_inner(pair, quote_volume);

        Some(())
    }

    /// Response description: https://docs.bitfinex.com/reference?ref=https://coder.social#rest-public-trades
    fn parse_last_trade_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let array = json.as_array()?;
        let array = array.get(2)?;
        let array = array.as_array()?;

        let last_trade_volume: f64 = array[2].as_f64()?;
        let last_trade_price: f64 = array[3].as_f64()?;
        self.parse_last_trade_json_inner(pair, last_trade_volume, last_trade_price);

        Some(())
    }

    /// Response description: https://docs.bitfinex.com/reference?ref=https://coder.social#rest-public-book
    fn parse_depth_json(&mut self, pair: String, json: serde_json::Value) -> Option<()> {
        let array = json.as_array()?;
        let array = array.get(1)?.as_array()?;

        if array.get(0)?.is_array() {
            let (asks, bids) = Self::depth_helper(array);
            self.parse_depth_json_inner(pair, asks, bids);
        }

        Some(())
    }
}
