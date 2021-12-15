use crate::worker::market_helpers::conversion_type::ConversionType;
use chrono::Utc;
use rustc_serialize::json::Json;

use crate::worker::market_helpers::market::Market;
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Bitfinex {
    pub spine: MarketSpine,
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

    fn get_websocket_url(&self, _pair: &str, _channel: MarketChannels) -> String {
        "wss://api-pub.bitfinex.com/ws/2".to_string()
    }

    fn get_websocket_on_open_msg(&self, pair: &str, channel: MarketChannels) -> Option<String> {
        Some(format!(
            "{{\"event\":\"subscribe\", \"channel\":\"{}\", \"symbol\":\"{}\"}}",
            self.get_channel_text_view(channel),
            pair
        ))
    }

    /// Response description: https://docs.bitfinex.com/reference?ref=https://coder.social#rest-public-ticker
    fn parse_ticker_info(&mut self, pair: String, info: String) {
        let json = Json::from_str(&info).unwrap();

        if let Some(array) = json.as_array() {
            if array.len() >= 2 {
                if let Some(array) = array[1].as_array() {
                    if array.len() >= 8 {
                        let currency = self.spine.get_pairs().get(&pair).unwrap().0.clone();
                        let conversion_coef: f64 = self
                            .spine
                            .get_conversion_coef(&currency, ConversionType::Crypto);

                        let volume: f64 = array[7].as_f64().unwrap();

                        trace!(
                            "called Bitfinex::parse_ticker_info(). Pair: {}, volume: {}",
                            pair,
                            volume,
                        );

                        self.spine.set_total_volume(&pair, volume * conversion_coef);
                    }
                }
            }
        }
    }

    /// Response description: https://docs.bitfinex.com/reference?ref=https://coder.social#rest-public-trades
    fn parse_last_trade_info(&mut self, pair: String, info: String) {
        let json = Json::from_str(&info).unwrap();

        if let Some(array) = json.as_array() {
            if array.len() >= 3 {
                if let Some(array) = array[2].as_array() {
                    if array.len() >= 4 {
                        let last_trade_volume: f64 = array[2].as_f64().unwrap().abs();
                        let last_trade_price: f64 = array[3].as_f64().unwrap();

                        trace!(
                            "called Bitfinex::parse_last_trade_info(). Pair: {}, last_trade_volume: {}, last_trade_price: {}",
                            pair,
                            last_trade_volume,
                            last_trade_price,
                        );

                        let conversion = self.spine.get_conversions().get(&pair).unwrap().clone();
                        let conversion_coef: f64 = if let ConversionType::None = conversion {
                            let currency = self.spine.get_pairs().get(&pair).unwrap().1.clone();
                            self.spine.get_conversion_coef(&currency, conversion)
                        } else {
                            1.0
                        };

                        self.spine.set_last_trade_volume(&pair, last_trade_volume);
                        self.spine
                            .set_last_trade_price(&pair, last_trade_price * conversion_coef);
                    }
                }
            }
        }
    }

    /// Response description: https://docs.bitfinex.com/reference?ref=https://coder.social#rest-public-book
    fn parse_depth_info(&mut self, pair: String, info: String) {
        let json = Json::from_str(&info).unwrap();

        if let Some(array) = json.as_array() {
            if array.len() >= 2 {
                if let Some(array) = array[1].as_array() {
                    if array.len() >= 3 {
                        if let Some(price) = array[0].as_f64() {
                            if let Some(amount) = array[2].as_f64() {
                                let mut ask_sum: f64 = self
                                    .spine
                                    .get_exchange_pairs()
                                    .get(&pair)
                                    .unwrap()
                                    .get_total_ask();

                                let mut bid_sum: f64 = self
                                    .spine
                                    .get_exchange_pairs()
                                    .get(&pair)
                                    .unwrap()
                                    .get_total_bid();

                                let conversion =
                                    self.spine.get_conversions().get(&pair).unwrap().clone();

                                let conversion_coef: f64 = if let ConversionType::None = conversion
                                {
                                    let currency =
                                        self.spine.get_pairs().get(&pair).unwrap().1.clone();

                                    self.spine.get_conversion_coef(&currency, conversion)
                                } else {
                                    1.0
                                };

                                if amount > 0.0 {
                                    // bid
                                    let x: f64 = -(price * amount);
                                    bid_sum += x * conversion_coef;
                                    self.spine.set_total_bid(&pair, bid_sum);

                                    trace!(
                                        "called Bitfinex::parse_depth_info(). Pair: {}, bid_sum: {}",
                                        pair,
                                        bid_sum,
                                    );
                                } else {
                                    // ask
                                    ask_sum += amount;
                                    self.spine.set_total_ask(&pair, ask_sum);

                                    trace!(
                                        "called Bitfinex::parse_depth_info(). Pair: {}, ask_sum: {}",
                                        pair,
                                        ask_sum,
                                    );
                                }

                                let timestamp = Utc::now();
                                self.spine
                                    .get_exchange_pairs_mut()
                                    .get_mut(&pair)
                                    .unwrap()
                                    .set_timestamp(timestamp);
                            }
                        }
                    }
                }
            }
        }
    }
}
