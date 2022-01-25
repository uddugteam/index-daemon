use crate::worker::helper_functions::strip_usd;
use crate::worker::market_helpers::conversion_type::ConversionType;
use crate::worker::market_helpers::exchange_pair::ExchangePair;
use crate::worker::market_helpers::exchange_pair_info::ExchangePairInfo;
use crate::worker::market_helpers::exchange_pair_info::ExchangePairInfoTrait;
use crate::worker::market_helpers::market::Market;
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::network_helpers::ws_server::ws_channel_response_payload::WsChannelResponsePayload;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use crate::worker::worker::{self, Worker};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

pub const EPS: f64 = 0.00001;

pub struct MarketSpine {
    worker: Arc<Mutex<Worker>>,
    pub arc: Option<Arc<Mutex<dyn Market + Send>>>,
    pub tx: Sender<JoinHandle<()>>,
    pub rest_timeout_sec: u64,
    pub name: String,
    mask_pairs: HashMap<String, String>,
    unmask_pairs: HashMap<String, String>,
    exchange_pairs: HashMap<String, ExchangePairInfo>,
    conversions: HashMap<String, ConversionType>,
    pairs: HashMap<String, (String, String)>,
    pub channels: Vec<MarketChannels>,
    pub ws_channels: WsChannels,
    pub graceful_shutdown: Arc<Mutex<bool>>,
}
impl MarketSpine {
    pub fn new(
        worker: Arc<Mutex<Worker>>,
        tx: Sender<JoinHandle<()>>,
        rest_timeout_sec: u64,
        name: String,
        channels: Option<Vec<MarketChannels>>,
        graceful_shutdown: Arc<Mutex<bool>>,
    ) -> Self {
        let channels = channels.unwrap_or(MarketChannels::get_all().to_vec());

        let channels = match name.as_str() {
            "poloniex" | "kucoin" => {
                // There is no distinct Trades channel in Poloniex. We get Trades inside of Book channel.

                // TODO: Implement Trades channel for Kucoin
                // Trades channel for Kucoin is not implemented.

                channels
                    .into_iter()
                    .filter(|v| !matches!(v, MarketChannels::Trades))
                    .collect()
            }
            "gemini" => {
                // Market Gemini has no channels (i.e. has single general channel), so we parse channel data from its single channel
                [MarketChannels::Ticker].to_vec()
            }
            _ => channels,
        };

        Self {
            worker,
            arc: None,
            tx,
            rest_timeout_sec,
            name,
            mask_pairs: HashMap::new(),
            unmask_pairs: HashMap::new(),
            exchange_pairs: HashMap::new(),
            conversions: HashMap::new(),
            pairs: HashMap::new(),
            channels,
            ws_channels: WsChannels::new(),
            graceful_shutdown,
        }
    }

    pub fn set_arc(&mut self, arc: Arc<Mutex<dyn Market + Send>>) {
        self.arc = Some(arc);
    }

    pub fn add_mask_pairs(&mut self, pairs: Vec<(&str, &str)>) {
        for pair in pairs {
            self.add_mask_pair(pair);
        }
    }

    pub fn add_mask_pair(&mut self, pair: (&str, &str)) {
        self.mask_pairs
            .insert(pair.0.to_string(), pair.1.to_string());
        self.unmask_pairs
            .insert(pair.1.to_string(), pair.0.to_string());
    }

    pub fn get_pairs(&self) -> &HashMap<String, (String, String)> {
        &self.pairs
    }

    pub fn get_conversions(&self) -> &HashMap<String, ConversionType> {
        &self.conversions
    }

    pub fn get_exchange_pairs(&self) -> &HashMap<String, ExchangePairInfo> {
        &self.exchange_pairs
    }

    pub fn get_exchange_pairs_mut(&mut self) -> &mut HashMap<String, ExchangePairInfo> {
        &mut self.exchange_pairs
    }

    pub fn add_exchange_pair(&mut self, pair_string: String, exchange_pair: ExchangePair) {
        self.exchange_pairs
            .insert(pair_string.clone(), ExchangePairInfo::new());
        self.conversions
            .insert(pair_string.clone(), exchange_pair.conversion);
        self.pairs
            .insert(pair_string, (exchange_pair.pair.0, exchange_pair.pair.1));
    }

    pub fn get_masked_value<'a>(&'a self, a: &'a str) -> &str {
        self.mask_pairs.get(a).map(|s| s.as_ref()).unwrap_or(a)
    }

    pub fn get_unmasked_value<'a>(&'a self, a: &'a str) -> &str {
        self.unmask_pairs.get(a).map(|s| s.as_ref()).unwrap_or(a)
    }

    /// TODO: Implement
    /// Cannot be implemented, because it depends on https://api.icex.ch/api/coins/ which is not working
    pub fn get_conversion_coef(&self, pair: &str) -> f64 {
        let conversion = *self.get_conversions().get(pair).unwrap();

        let _currency = match conversion {
            ConversionType::None => Some(self.get_pairs().get(pair).unwrap().1.clone()),
            ConversionType::Crypto => Some(self.get_pairs().get(pair).unwrap().0.clone()),
            _ => None,
        };

        1.0
    }

    pub fn get_total_volume(&self, pair: &str) -> f64 {
        if self.exchange_pairs.contains_key(pair) {
            self.exchange_pairs.get(pair).unwrap().get_total_volume()
        } else {
            0.0
        }
    }

    pub fn set_total_volume(&mut self, pair: &str, value: f64) {
        if value != 0.0 {
            let old_value: f64 = self.exchange_pairs.get(pair).unwrap().get_total_volume();

            if (old_value - value).abs() > EPS {
                let timestamp = Utc::now();
                self.exchange_pairs
                    .get_mut(pair)
                    .unwrap()
                    .set_total_volume(value, timestamp);

                self.update_market_pair(pair, "totalValues", false);

                let pair_tuple = self.pairs.get(pair).unwrap();
                if let Some(coin) = strip_usd(pair_tuple) {
                    let response_payload = WsChannelResponsePayload::CoinExchangeVolume {
                        coin,
                        value,
                        exchange: self.name.clone(),
                        timestamp,
                    };

                    self.ws_channels.send(response_payload);
                }

                self.recalculate_total_volume(pair_tuple.0.clone());
            }
        }
    }

    fn recalculate_total_volume(&self, currency: String) {
        let worker = Arc::clone(&self.worker);

        let thread_name = format!(
            "fn: recalculate_total_volume, market: {}, currency: {}",
            self.name, currency,
        );
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                if worker::is_graceful_shutdown(&worker) {
                    return;
                }

                worker.lock().unwrap().recalculate_total_volume(currency);
            })
            .unwrap();
        self.tx.send(thread).unwrap();
    }

    fn recalculate_pair_average_price(&self, pair: (String, String), new_price: f64) {
        let worker = Arc::clone(&self.worker);

        let thread_name = format!(
            "fn: recalculate_pair_average_price, market: {}, pair: {:?}",
            self.name, pair,
        );
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                if worker::is_graceful_shutdown(&worker) {
                    return;
                }

                worker
                    .lock()
                    .unwrap()
                    .recalculate_pair_average_price(pair, new_price);
            })
            .unwrap();
        self.tx.send(thread).unwrap();
    }

    // TODO: Implement
    pub fn update_market_pair(&mut self, _pair: &str, _scope: &str, _price_changed: bool) {}

    pub fn set_last_trade_volume(&mut self, pair: &str, value: f64) {
        if value != 0.0 {
            let old_value: f64 = self
                .exchange_pairs
                .get(pair)
                .unwrap()
                .get_last_trade_volume();

            if (old_value - value).abs() > EPS {
                self.exchange_pairs
                    .get_mut(pair)
                    .unwrap()
                    .set_last_trade_volume(value);

                self.update_market_pair(pair, "lastTrade", false);
            }
        }
    }

    pub fn set_last_trade_price(&mut self, pair: &str, value: f64) {
        let old_value: f64 = self
            .exchange_pairs
            .get(pair)
            .unwrap()
            .get_last_trade_price();

        // If new value is a Real price
        if value <= 0.0 {
            return;
        }
        // If old_value was defined
        if old_value > 0.0 {
            // If new value is not equal to the old value
            if (old_value - value).abs() < EPS {
                return;
            }
            // If new value is inside Real sequence
            if value > old_value * 1.5 || value < old_value / 1.5 {
                return;
            }
        }

        let timestamp = Utc::now();
        self.exchange_pairs
            .get_mut(pair)
            .unwrap()
            .set_last_trade_price(value, timestamp);

        let pair_tuple = self.pairs.get(pair).unwrap().clone();
        if let Some(coin) = strip_usd(&pair_tuple) {
            let response_payload = WsChannelResponsePayload::CoinExchangePrice {
                coin,
                value,
                exchange: self.name.clone(),
                timestamp,
            };

            self.ws_channels.send(response_payload);
        }

        self.recalculate_pair_average_price(pair_tuple, value);

        self.update_market_pair(pair, "lastTrade", false);
    }

    pub fn set_total_ask(&mut self, pair: &str, value: f64) {
        let old_value: f64 = self.get_exchange_pairs().get(pair).unwrap().get_total_ask();

        if (old_value - value).abs() > EPS {
            self.get_exchange_pairs_mut()
                .get_mut(pair)
                .unwrap()
                .set_total_ask(value);

            self.update_market_pair(pair, "totalValues", false);
        }
    }

    pub fn set_total_bid(&mut self, pair: &str, value: f64) {
        let old_value: f64 = self.get_exchange_pairs().get(pair).unwrap().get_total_bid();

        if (old_value - value).abs() > EPS {
            self.get_exchange_pairs_mut()
                .get_mut(pair)
                .unwrap()
                .set_total_bid(value);

            self.update_market_pair(pair, "totalValues", false);
        }
    }
}

#[cfg(test)]
pub mod test {
    use crate::worker::market_helpers::conversion_type::ConversionType;
    use crate::worker::market_helpers::exchange_pair::ExchangePair;
    use crate::worker::market_helpers::market_spine::MarketSpine;
    use crate::worker::worker::test::{check_threads, make_worker};
    use ntest::timeout;
    use std::sync::mpsc::Receiver;
    use std::sync::{Arc, Mutex};
    use std::thread::JoinHandle;

    pub fn make_spine(market_name: Option<&str>) -> (MarketSpine, Receiver<JoinHandle<()>>) {
        let market_name = market_name.unwrap_or("binance").to_string();
        let (worker, tx, rx) = make_worker();
        let graceful_shutdown = Arc::new(Mutex::new(false));

        let spine = MarketSpine::new(worker, tx, 1, market_name, None, graceful_shutdown);

        (spine, rx)
    }

    #[test]
    fn test_add_exchange_pair() {
        let (mut spine, _) = make_spine(None);

        let pair_string = "some_pair_string".to_string();

        let pair_tuple = ("some_coin_1".to_string(), "some_coin_2".to_string());
        let conversion_type = ConversionType::Crypto;
        let exchange_pair = ExchangePair {
            pair: pair_tuple.clone(),
            conversion: conversion_type,
        };

        spine.add_exchange_pair(pair_string.clone(), exchange_pair);

        assert!(spine.get_exchange_pairs().get(&pair_string).is_some());

        assert_eq!(
            spine.get_conversions().get(&pair_string).unwrap(),
            &conversion_type
        );

        assert_eq!(spine.get_pairs().get(&pair_string).unwrap(), &pair_tuple);
    }

    #[test]
    #[timeout(1000)]
    fn test_recalculate_total_volume() {
        let market_name = "binance";
        let currency = "ABC".to_string();

        let (spine, rx) = make_spine(Some(market_name));

        let thread_names = vec![format!(
            "fn: recalculate_total_volume, market: {}, currency: {}",
            market_name, currency,
        )];

        spine.recalculate_total_volume(currency);
        check_threads(thread_names, rx);
    }

    #[test]
    #[timeout(1000)]
    fn test_recalculate_pair_average_price() {
        let market_name = "binance";
        let pair = ("ABC".to_string(), "DEF".to_string());
        let new_price = 100.0;

        let (spine, rx) = make_spine(Some(market_name));

        let thread_names = vec![format!(
            "fn: recalculate_pair_average_price, market: {}, pair: {:?}",
            market_name, pair,
        )];

        spine.recalculate_pair_average_price(pair, new_price);
        check_threads(thread_names, rx);
    }
}
