use crate::worker::market_helpers::conversion_type::ConversionType;
use chrono::Utc;
use rustc_serialize::json::Json;

use crate::worker::market_helpers::market::Market;
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Binance {
    pub spine: MarketSpine,
}

impl Market for Binance {
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
            MarketChannels::Ticker => "ticker".to_string(),
            MarketChannels::Trades => "trade".to_string(),
            MarketChannels::Book => "depth20".to_string(),
        }
    }

    fn get_websocket_url(&self, pair: &str, channel: MarketChannels) -> String {
        format!(
            "wss://stream.binance.com:9443/ws/{}@{}",
            pair,
            self.get_channel_text_view(channel)
        )
    }

    fn get_websocket_on_open_msg(&self, pair: &str, channel: MarketChannels) -> Option<String> {
        None
    }

    fn parse_ticker_info__socket(&mut self, pair: String, info: String) {
        let json = Json::from_str(&info).unwrap();

        if let Some(object) = json.as_object() {
            if let Some(volume) = object.get("v") {
                if let Some(volume) = volume.as_string() {
                    if let Ok(volume) = volume.parse::<f64>() {
                        println!("called Binance::parse_ticker_info__socket()");
                        println!("pair: {}", pair);
                        println!("volume: {}", volume);

                        let currency = self.spine.get_pairs().get(&pair).unwrap().0.clone();

                        let conversion_coef: f64 = self
                            .spine
                            .get_conversion_coef(&currency, ConversionType::Crypto);

                        self.spine.set_total_volume(&pair, volume * conversion_coef);
                    }
                }
            }
        }
    }

    fn parse_last_trade_info__socket(&mut self, pair: String, info: String) {
        let json = Json::from_str(&info).unwrap();

        if let Some(object) = json.as_object() {
            println!("called Binance::parse_last_trade_info__socket()");
            println!("pair: {}", pair);

            let last_trade_volume: f64 = object
                .get("q")
                .unwrap()
                .as_string()
                .unwrap()
                .parse()
                .unwrap();
            println!("last_trade_volume: {}", last_trade_volume);

            let last_trade_price: f64 = object
                .get("p")
                .unwrap()
                .as_string()
                .unwrap()
                .parse()
                .unwrap();
            println!("last_trade_price: {}", last_trade_price);

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

    fn parse_depth_info__socket(&mut self, pair: String, info: String) {
        let json = Json::from_str(&info).unwrap();

        if let Some(object) = json.as_object() {
            println!("called Binance::parse_depth_info__socket()");
            println!("pair: {}", pair);

            let conversion = self.spine.get_conversions().get(&pair).unwrap().clone();

            let conversion_coef: f64 = if let ConversionType::None = conversion {
                let currency = self.spine.get_pairs().get(&pair).unwrap().1.clone();

                self.spine.get_conversion_coef(&currency, conversion)
            } else {
                1.0
            };

            if let Some(asks) = object.get("asks") {
                if let Some(asks) = asks.as_array() {
                    let mut ask_sum: f64 = 0.0;

                    for ask in asks {
                        if let Some(ask) = ask.as_array() {
                            let ask = ask[1].as_string().unwrap().parse::<f64>().unwrap();
                            ask_sum += ask;
                        }
                    }
                    println!("ask_sum: {}", ask_sum);

                    self.spine.set_total_ask(&pair, ask_sum);
                }
            }

            if let Some(bids) = object.get("bids") {
                if let Some(bids) = bids.as_array() {
                    let mut bid_sum: f64 = 0.0;

                    for bid in bids {
                        if let Some(bid) = bid.as_array() {
                            let price = bid[0].as_string().unwrap().parse::<f64>().unwrap();
                            let size = bid[1].as_string().unwrap().parse::<f64>().unwrap();
                            bid_sum += size * price;
                        }
                    }

                    bid_sum *= conversion_coef;
                    println!("bid_sum: {}", bid_sum);

                    self.spine.set_total_bid(&pair, bid_sum);
                }
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