use crate::repository::repositories::MarketRepositoriesByMarketValue;
use crate::worker::market_helpers::exchange_pair_info::ExchangePairInfo;
use crate::worker::market_helpers::market_channels::ExternalMarketChannels;
use crate::worker::market_helpers::pair_average_price::StoredAndWsTransmissibleF64ByPairTuple;
use crate::worker::market_helpers::percent_change::PercentChangeByInterval;
use crate::worker::market_helpers::stored_and_ws_transmissible_f64::StoredAndWsTransmissibleF64;
use crate::worker::network_helpers::ws_server::holders::helper_functions::HolderHashMap;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

pub const EPS: f64 = 0.00001;

pub struct MarketSpine {
    pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
    index_price: Arc<RwLock<StoredAndWsTransmissibleF64>>,
    pub tx: Sender<JoinHandle<()>>,
    pub rest_timeout_sec: u64,
    pub name: String,
    mask_pairs: HashMap<String, String>,
    unmask_pairs: HashMap<String, String>,
    exchange_pairs: HashMap<String, ExchangePairInfo>,
    index_pairs: Vec<(String, String)>,
    pairs: HashMap<String, (String, String)>,
    pub channels: Vec<ExternalMarketChannels>,
    pub graceful_shutdown: Arc<RwLock<bool>>,
}
impl MarketSpine {
    pub fn new(
        pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
        index_price: Arc<RwLock<StoredAndWsTransmissibleF64>>,
        index_pairs: Vec<(String, String)>,
        tx: Sender<JoinHandle<()>>,
        rest_timeout_sec: u64,
        name: String,
        channels: Vec<ExternalMarketChannels>,
        graceful_shutdown: Arc<RwLock<bool>>,
    ) -> Self {
        let channels = match name.as_str() {
            "poloniex" | "kucoin" => {
                // There is no distinct Trades channel in Poloniex. We get Trades inside of Book channel.

                // TODO: Implement Trades channel for Kucoin
                // Trades channel for Kucoin is not implemented.

                channels
                    .into_iter()
                    .filter(|v| !matches!(v, ExternalMarketChannels::Trades))
                    .collect()
            }
            "gemini" => {
                // Market Gemini has no channels (i.e. has single general channel), so we parse channel data from its single channel
                [ExternalMarketChannels::Ticker].to_vec()
            }
            _ => channels,
        };

        Self {
            pair_average_price,
            index_price,
            tx,
            rest_timeout_sec,
            name,
            mask_pairs: HashMap::new(),
            unmask_pairs: HashMap::new(),
            exchange_pairs: HashMap::new(),
            index_pairs,
            pairs: HashMap::new(),
            channels,
            graceful_shutdown,
        }
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

    pub fn get_exchange_pairs(&self) -> &HashMap<String, ExchangePairInfo> {
        &self.exchange_pairs
    }

    pub fn get_exchange_pairs_mut(&mut self) -> &mut HashMap<String, ExchangePairInfo> {
        &mut self.exchange_pairs
    }

    pub fn add_exchange_pair(
        &mut self,
        pair_string: String,
        exchange_pair: (String, String),
        repositories: Option<MarketRepositoriesByMarketValue>,
        percent_change_holder: &HolderHashMap<PercentChangeByInterval>,
        percent_change_interval_sec: u64,
        ws_channels_holder: &HolderHashMap<WsChannels>,
    ) {
        self.exchange_pairs.insert(
            pair_string.clone(),
            ExchangePairInfo::new(
                repositories,
                percent_change_holder,
                percent_change_interval_sec,
                ws_channels_holder,
                self.name.clone(),
                exchange_pair.clone(),
            ),
        );
        self.pairs
            .insert(pair_string, (exchange_pair.0, exchange_pair.1));
    }

    pub fn get_masked_value<'a>(&'a self, a: &'a str) -> &str {
        self.mask_pairs.get(a).map(|s| s.as_ref()).unwrap_or(a)
    }

    pub fn get_unmasked_value<'a>(&'a self, a: &'a str) -> &str {
        self.unmask_pairs.get(a).map(|s| s.as_ref()).unwrap_or(a)
    }

    pub fn set_total_volume(&mut self, pair: &str, value: f64) {
        if value != 0.0 {
            let old_value: f64 = self
                .exchange_pairs
                .get(pair)
                .unwrap()
                .total_volume
                .get()
                .unwrap_or(0.0);

            if (old_value - value).abs() > EPS {
                self.exchange_pairs
                    .get_mut(pair)
                    .unwrap()
                    .total_volume
                    .set(value);
            }
        }
    }

    fn recalculate_pair_average_price(&self, pair: (String, String), new_price: f64) {
        let pair_average_price_2 = Arc::clone(self.pair_average_price.get(&pair).unwrap());
        let pair_average_price_3 = self.pair_average_price.clone();
        let index_price_2 = Arc::clone(&self.index_price);
        let index_pairs_2 = self.index_pairs.clone();
        let tx_2 = self.tx.clone();
        let market_name_2 = self.name.to_string();

        let thread_name = format!(
            "fn: recalculate_pair_average_price, market: {}, pair: {:?}",
            market_name_2, pair,
        );
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                if let Ok(mut pair_average_price_2) = pair_average_price_2.write() {
                    let old_avg = pair_average_price_2.get().unwrap_or(new_price);

                    let new_avg = (new_price + old_avg) / 2.0;

                    info!("new {}-{} average trade price: {}", pair.0, pair.1, new_avg);

                    pair_average_price_2.set(new_avg);

                    Self::recalculate_index_price(
                        market_name_2,
                        pair,
                        tx_2,
                        pair_average_price_3,
                        index_price_2,
                        index_pairs_2,
                    );
                }
            })
            .unwrap();
        let _ = self.tx.send(thread);
    }

    fn recalculate_index_price(
        market_name: String,
        pair: (String, String),
        tx: Sender<JoinHandle<()>>,
        pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
        index_price: Arc<RwLock<StoredAndWsTransmissibleF64>>,
        index_pairs: Vec<(String, String)>,
    ) {
        let thread_name = format!(
            "fn: recalculate_index_price, market: {}, pair: {:?}",
            market_name, pair,
        );
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                let mut sum = 0.0;
                let mut count = 0;
                for index_pair in index_pairs {
                    let value = pair_average_price
                        .get(&index_pair)
                        .unwrap()
                        .read()
                        .unwrap()
                        .get();

                    if let Some(value) = value {
                        count += 1;
                        sum += value;
                    }
                }

                // Do not calculate if no coins or only one coin
                if count > 1 {
                    let avg = sum / count as f64;

                    info!("new index price: {}", avg);

                    index_price.write().unwrap().set(avg);
                }
            })
            .unwrap();
        let _ = tx.send(thread);
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
            .get()
            .unwrap_or(0.0);

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
            .set(value);

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
    use crate::worker::market_helpers::market_spine::MarketSpine;
    use crate::worker::worker::test::prepare_worker_params;
    use std::sync::mpsc::Receiver;
    use std::sync::{Arc, RwLock};
    use std::thread::JoinHandle;

    pub fn make_spine(market_name: Option<&str>) -> (MarketSpine, Receiver<JoinHandle<()>>) {
        let market_name = market_name.unwrap_or("binance").to_string();
        let (tx, rx, _) = prepare_worker_params();
        let graceful_shutdown = Arc::new(RwLock::new(false));

        let config = ConfigScheme::default();
        let RepositoriesPrepared {
            index_price_repository: _,
            pair_average_price_repositories: _,
            market_repositories: _,
            percent_change_holder: _,
            ws_channels_holder: _,
            index_price,
            pair_average_price,
        } = RepositoriesPrepared::make(&config);

        let spine = MarketSpine::new(
            pair_average_price,
            index_price,
            config.market.index_pairs,
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
        let exchange_pair = pair_tuple.clone();

        let mut config = ConfigScheme::default();
        config.market.exchange_pairs = vec![exchange_pair.clone()];

        let RepositoriesPrepared {
            index_price_repository: _,
            pair_average_price_repositories: _,
            market_repositories,
            percent_change_holder,
            ws_channels_holder,
            index_price: _,
            pair_average_price: _,
        } = RepositoriesPrepared::make(&config);

        let pair_string = "some_pair_string".to_string();

        spine.add_exchange_pair(
            pair_string.clone(),
            exchange_pair.clone(),
            market_repositories.map(|mut v| {
                v.remove(&market_name)
                    .unwrap()
                    .remove(&exchange_pair)
                    .unwrap()
            }),
            &percent_change_holder,
            config.service.percent_change_interval_sec,
            &ws_channels_holder,
        );

        assert!(spine.get_exchange_pairs().get(&pair_string).is_some());

        assert_eq!(spine.get_pairs().get(&pair_string).unwrap(), &pair_tuple);
    }
}
