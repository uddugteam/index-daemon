use crate::repository::repositories::RepositoriesByMarketValue;
use crate::worker::market_helpers::conversion_type::ConversionType;
use crate::worker::market_helpers::exchange_pair::ExchangePair;
use crate::worker::market_helpers::exchange_pair_info::ExchangePairInfo;
use crate::worker::market_helpers::market::Market;
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::pair_average_price::PairAveragePriceType;
use crate::worker::network_helpers::ws_server::ws_channels_holder::WsChannelsHolderHashMap;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

pub const EPS: f64 = 0.00001;

pub struct MarketSpine {
    pair_average_price: PairAveragePriceType,
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
    pub graceful_shutdown: Arc<Mutex<bool>>,
}
impl MarketSpine {
    pub fn new(
        pair_average_price: PairAveragePriceType,
        tx: Sender<JoinHandle<()>>,
        rest_timeout_sec: u64,
        name: String,
        channels: Vec<MarketChannels>,
        graceful_shutdown: Arc<Mutex<bool>>,
    ) -> Self {
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
            pair_average_price,
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

    pub fn add_exchange_pair(
        &mut self,
        pair_string: String,
        exchange_pair: ExchangePair,
        repositories: Option<RepositoriesByMarketValue>,
        ws_channels_holder: &WsChannelsHolderHashMap,
    ) {
        self.exchange_pairs.insert(
            pair_string.clone(),
            ExchangePairInfo::new(
                repositories,
                ws_channels_holder,
                self.name.clone(),
                exchange_pair.pair.clone(),
            ),
        );
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

    pub fn get_total_volume(&self, pair: &str) -> f64 {
        if self.exchange_pairs.contains_key(pair) {
            self.exchange_pairs
                .get(pair)
                .unwrap()
                .total_volume
                .get_value()
        } else {
            0.0
        }
    }

    pub fn set_total_volume(&mut self, pair: &str, value: f64) {
        if value != 0.0 {
            let old_value: f64 = self
                .exchange_pairs
                .get(pair)
                .unwrap()
                .total_volume
                .get_value();

            if (old_value - value).abs() > EPS {
                self.exchange_pairs
                    .get_mut(pair)
                    .unwrap()
                    .total_volume
                    .set_new_value(value);
            }
        }
    }

    fn recalculate_pair_average_price(&self, pair: (String, String), new_price: f64) {
        let pair_average_price_2 = Arc::clone(self.pair_average_price.get(&pair).unwrap());

        let thread_name = format!(
            "fn: recalculate_pair_average_price, market: {}, pair: {:?}",
            self.name, pair,
        );
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                if let Ok(mut pair_average_price_2) = pair_average_price_2.lock() {
                    let old_avg = pair_average_price_2.get_value();

                    let new_avg = (new_price + old_avg) / 2.0;

                    info!("new {}-{} average trade price: {}", pair.0, pair.1, new_avg);

                    pair_average_price_2.set_new_value(new_avg);
                }
            })
            .unwrap();
        self.tx.send(thread).unwrap();
    }

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
            }
        }
    }

    pub fn set_last_trade_price(&mut self, pair: &str, value: f64) {
        let old_value: f64 = self
            .exchange_pairs
            .get(pair)
            .unwrap()
            .last_trade_price
            .get_value();

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

        self.exchange_pairs
            .get_mut(pair)
            .unwrap()
            .last_trade_price
            .set_new_value(value);

        let pair_tuple = self.pairs.get(pair).unwrap().clone();
        self.recalculate_pair_average_price(pair_tuple, value);
    }

    pub fn set_total_ask(&mut self, pair: &str, value: f64) {
        let old_value: f64 = self.get_exchange_pairs().get(pair).unwrap().get_total_ask();

        if (old_value - value).abs() > EPS {
            self.get_exchange_pairs_mut()
                .get_mut(pair)
                .unwrap()
                .set_total_ask(value);
        }
    }

    pub fn set_total_bid(&mut self, pair: &str, value: f64) {
        let old_value: f64 = self.get_exchange_pairs().get(pair).unwrap().get_total_bid();

        if (old_value - value).abs() > EPS {
            self.get_exchange_pairs_mut()
                .get_mut(pair)
                .unwrap()
                .set_total_bid(value);
        }
    }
}

#[cfg(test)]
pub mod test {
    use crate::config_scheme::config_scheme::ConfigScheme;
    use crate::config_scheme::repositories_prepared::RepositoriesPrepared;
    use crate::worker::market_helpers::conversion_type::ConversionType;
    use crate::worker::market_helpers::exchange_pair::ExchangePair;
    use crate::worker::market_helpers::market_spine::MarketSpine;
    use crate::worker::worker::test::make_worker;
    use std::sync::mpsc::Receiver;
    use std::sync::{Arc, Mutex};
    use std::thread::JoinHandle;

    pub fn make_spine(market_name: Option<&str>) -> (MarketSpine, Receiver<JoinHandle<()>>) {
        let market_name = market_name.unwrap_or("binance").to_string();
        let (_worker, tx, rx) = make_worker();
        let graceful_shutdown = Arc::new(Mutex::new(false));

        let config = ConfigScheme::default();
        let RepositoriesPrepared {
            pair_average_price_repository: _,
            market_repositories: _,
            ws_channels_holder: _,
            pair_average_price,
        } = RepositoriesPrepared::make(&config);

        let spine = MarketSpine::new(
            pair_average_price,
            tx,
            1,
            market_name,
            config.market.channels,
            graceful_shutdown,
        );

        (spine, rx)
    }

    #[test]
    fn test_add_exchange_pair() {
        let (mut spine, _) = make_spine(None);
        let market_name = spine.name.clone();

        let pair_tuple = ("some_coin_1".to_string(), "some_coin_2".to_string());
        let conversion_type = ConversionType::Crypto;
        let exchange_pair = ExchangePair {
            pair: pair_tuple.clone(),
            conversion: conversion_type,
        };

        let mut config = ConfigScheme::default();
        config.market.exchange_pairs = vec![exchange_pair.clone()];

        let RepositoriesPrepared {
            pair_average_price_repository: _,
            market_repositories,
            ws_channels_holder,
            pair_average_price: _,
        } = RepositoriesPrepared::make(&config);

        let pair_string = "some_pair_string".to_string();

        spine.add_exchange_pair(
            pair_string.clone(),
            exchange_pair.clone(),
            market_repositories.map(|mut v| {
                v.remove(&market_name)
                    .unwrap()
                    .remove(&exchange_pair.pair)
                    .unwrap()
            }),
            &ws_channels_holder,
        );

        assert!(spine.get_exchange_pairs().get(&pair_string).is_some());

        assert_eq!(
            spine.get_conversions().get(&pair_string).unwrap(),
            &conversion_type
        );

        assert_eq!(spine.get_pairs().get(&pair_string).unwrap(), &pair_tuple);
    }
}
