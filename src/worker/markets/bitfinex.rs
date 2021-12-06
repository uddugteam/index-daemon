use crate::worker::market_helpers::conversion_type::ConversionType;
use chrono::Utc;
use rustc_serialize::json::Json;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

use crate::worker::market_helpers::market::Market;
use crate::worker::market_helpers::market_spine::MarketSpine;
use crate::worker::network_helpers::socket_helper::SocketHelper;

pub struct Bitfinex {
    pub spine: MarketSpine,
}

impl Bitfinex {
    pub fn subscribe_channel(market: Arc<Mutex<dyn Market + Send>>, pair: String, channel: &str) {
        // println!("called Bitfinex::subscribe_channel()");

        let socker_helper = SocketHelper::new(
            "wss://api-pub.bitfinex.com/ws/2".to_string(),
            format!(
                "{{\"event\":\"subscribe\", \"channel\":\"{}\", \"symbol\":\"{}\"}}",
                channel, pair
            ),
            pair,
            |pair: String, info: String| match channel {
                "ticker" => market.lock().unwrap().parse_ticker_info__socket(pair, info),
                "trades" => market
                    .lock()
                    .unwrap()
                    .parse_last_trade_info__socket(pair, info),
                "book" => market.lock().unwrap().parse_depth_info__socket(pair, info),
                _ => println!("Error: channel not supported."),
            },
        );
        socker_helper.start();
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

    // TODO: Replace `delay` constants with parameters
    fn update(&mut self) {
        // println!("called Bitfinex::update()");

        self.spine.socket_enabled = true;

        let channels = ["ticker", "trades", "book"];

        for exchange_pair in self.spine.get_exchange_pairs() {
            for channel in channels {
                let market_2 = Arc::clone(self.spine.arc.as_ref().unwrap());
                let pair_2 = exchange_pair.0.to_string();
                let thread = thread::spawn(move || {
                    let market_3 = Arc::clone(&market_2);
                    let pair_3 = pair_2.clone();
                    loop {
                        Self::subscribe_channel(Arc::clone(&market_3), pair_3.clone(), channel);
                        thread::sleep(time::Duration::from_millis(10000));
                    }
                });
                thread::sleep(time::Duration::from_millis(3000));

                self.spine.tx.send(thread).unwrap();
            }
        }
    }

    /// Response description: https://docs.bitfinex.com/reference?ref=https://coder.social#rest-public-ticker
    fn parse_ticker_info__socket(&mut self, pair: String, info: String) {
        let json = Json::from_str(&info).unwrap();

        if let Some(array) = json.as_array() {
            if array.len() >= 2 {
                if let Some(array) = array[1].as_array() {
                    if array.len() >= 8 {
                        // println!("called Bitfinex::parse_ticker_info__socket()");
                        // println!("pair: {}", pair);
                        // println!("json: {}", json);

                        let currency = self.spine.get_pairs().get(&pair).unwrap().0.clone();

                        let conversion_coef: f64 = self
                            .spine
                            .get_conversion_coef(&currency, ConversionType::Crypto);

                        let volume: f64 = array[7].as_f64().unwrap();
                        // println!("volume: {}", volume);

                        self.spine.set_total_volume(&pair, volume * conversion_coef);
                    }
                }
            }
        }
    }

    /// Response description: https://docs.bitfinex.com/reference?ref=https://coder.social#rest-public-trades
    fn parse_last_trade_info__socket(&mut self, pair: String, info: String) {
        let json = Json::from_str(&info).unwrap();

        if let Some(array) = json.as_array() {
            if array.len() >= 3 {
                if let Some(array) = array[2].as_array() {
                    if array.len() >= 4 {
                        // println!("called Bitfinex::parse_last_trade_info__socket()");

                        let last_trade_volume: f64 = array[2].as_f64().unwrap().abs();
                        let last_trade_price: f64 = array[3].as_f64().unwrap();

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
    fn parse_depth_info__socket(&mut self, pair: String, info: String) {
        let json = Json::from_str(&info).unwrap();

        if let Some(array) = json.as_array() {
            if array.len() >= 2 {
                if let Some(array) = array[1].as_array() {
                    if array.len() >= 3 {
                        if let Some(price) = array[0].as_f64() {
                            if let Some(amount) = array[2].as_f64() {
                                // println!("called Bitfinex::parse_depth_info__socket()");

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

                                let timestamp = Utc::now();

                                if amount > 0.0 {
                                    // bid
                                    let x: f64 = -(price * amount);

                                    bid_sum += x * conversion_coef;

                                    self.spine.set_total_bid(&pair, bid_sum);
                                } else {
                                    // ask
                                    ask_sum += amount;

                                    self.spine.set_total_ask(&pair, ask_sum);
                                }

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
