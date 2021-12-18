use crate::worker::market_helpers::conversion_type::ConversionType;
use chrono::Utc;
use rustc_serialize::json::Json;

use crate::worker::market_helpers::market::Market;
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Coinbase {
    pub spine: MarketSpine,
}

impl Market for Coinbase {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn make_pair(&self, pair: (&str, &str)) -> String {
        (self.spine.get_masked_value(pair.0).to_string()
            + "-"
            + self.spine.get_masked_value(pair.1))
        .to_uppercase()
    }

    fn get_channel_text_view(&self, channel: MarketChannels) -> String {
        match channel {
            MarketChannels::Ticker => "ticker".to_string(),
            MarketChannels::Trades => "matches".to_string(),
            MarketChannels::Book => "level2".to_string(),
            // MarketChannels::Book => "full".to_string(),
        }
    }

    fn get_websocket_url(&self, _pair: &str, _channel: MarketChannels) -> String {
        "wss://ws-feed.pro.coinbase.com".to_string()
    }

    fn get_websocket_on_open_msg(&self, pair: &str, channel: MarketChannels) -> Option<String> {
        Some(format!(
            "{{\"type\": \"subscribe\", \"product_ids\": [\"{}\"], \"channels\": [\"{}\"]}}",
            pair,
            self.get_channel_text_view(channel)
        ))
    }

    // TODO: Check whether key `volume_24h` is right
    fn parse_ticker_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(object) = json.as_object() {
                if let Some(volume) = object.get("volume_24h") {
                    if let Some(volume) = volume.as_string() {
                        if let Ok(volume) = volume.parse::<f64>() {
                            info!("new {} ticker on Coinbase with volume: {}", pair, volume,);

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
    }

    fn parse_last_trade_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(object) = json.as_object() {
                if let Some(last_trade_volume) = object.get("size") {
                    if let Some(last_trade_price) = object.get("price") {
                        let last_trade_volume: f64 =
                            last_trade_volume.as_string().unwrap().parse().unwrap();

                        let last_trade_price: f64 =
                            last_trade_price.as_string().unwrap().parse().unwrap();

                        info!(
                            "new {} trade on Coinbase with volume: {}, price: {}",
                            pair, last_trade_volume, last_trade_price,
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

    fn parse_depth_info(&mut self, pair: String, info: String) {
        if let Ok(json) = Json::from_str(&info) {
            if let Some(object) = json.as_object() {
                if let Some(asks) = object.get("asks") {
                    if let Some(bids) = object.get("bids") {
                        let conversion = self.spine.get_conversions().get(&pair).unwrap().clone();
                        let conversion_coef: f64 = if let ConversionType::None = conversion {
                            let currency = self.spine.get_pairs().get(&pair).unwrap().1.clone();
                            self.spine.get_conversion_coef(&currency, conversion)
                        } else {
                            1.0
                        };

                        let asks = asks.as_array().unwrap();
                        let mut ask_sum: f64 = 0.0;
                        for ask in asks {
                            if let Some(ask) = ask.as_array() {
                                let ask = ask[1].as_string().unwrap().parse::<f64>().unwrap();
                                ask_sum += ask;
                            }
                        }

                        let bids = bids.as_array().unwrap();
                        let mut bid_sum: f64 = 0.0;
                        for bid in bids {
                            if let Some(bid) = bid.as_array() {
                                let price = bid[0].as_string().unwrap().parse::<f64>().unwrap();
                                let size = bid[1].as_string().unwrap().parse::<f64>().unwrap();
                                bid_sum += size * price;
                            }
                        }
                        bid_sum *= conversion_coef;

                        info!(
                            "new {} book on Coinbase with ask_sum: {}, bid_sum: {}",
                            pair, ask_sum, bid_sum,
                        );

                        self.spine.set_total_ask(&pair, ask_sum);
                        self.spine.set_total_bid(&pair, bid_sum);

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
