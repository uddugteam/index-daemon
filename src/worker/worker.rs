use crate::config_scheme::config_scheme::ConfigScheme;
use crate::config_scheme::market_config::MarketConfig;
use crate::config_scheme::service_config::ServiceConfig;
use crate::repository::repositories::{
    RepositoriesByMarketName, RepositoryForF64ByTimestampAndPairTuple,
};
use crate::worker::market_helpers::exchange_pair::ExchangePair;
use crate::worker::market_helpers::market::{market_factory, Market};
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;
use crate::worker::market_helpers::stored_and_ws_transmissible_f64_by_pair_tuple::StoredAndWsTransmissibleF64ByPairTuple;
use crate::worker::network_helpers::ws_server::ws_channel_request::WsChannelRequest;
use crate::worker::network_helpers::ws_server::ws_channel_response_sender::WsChannelResponseSender;
use crate::worker::network_helpers::ws_server::ws_server::WsServer;
use chrono::{DateTime, Utc, MIN_DATETIME};
use reqwest::blocking::multipart::{Form, Part};
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time;

pub struct Worker {
    arc: Option<Arc<Mutex<Self>>>,
    tx: Sender<JoinHandle<()>>,
    graceful_shutdown: Arc<Mutex<bool>>,
    markets: HashMap<String, Arc<Mutex<dyn Market + Send>>>,
    market_names_by_ws_channel_key: HashMap<(String, String), Vec<String>>,
    pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
    capitalization: HashMap<String, f64>,
    last_capitalization_refresh: DateTime<Utc>,
}

pub fn is_graceful_shutdown(worker: &Arc<Mutex<Worker>>) -> bool {
    *worker.lock().unwrap().graceful_shutdown.lock().unwrap()
}

impl Worker {
    pub fn new(
        tx: Sender<JoinHandle<()>>,
        graceful_shutdown: Arc<Mutex<bool>>,
        pair_average_price_repository: Option<RepositoryForF64ByTimestampAndPairTuple>,
    ) -> Arc<Mutex<Self>> {
        let worker = Worker {
            arc: None,
            tx,
            graceful_shutdown,
            markets: HashMap::new(),
            market_names_by_ws_channel_key: HashMap::new(),
            pair_average_price: StoredAndWsTransmissibleF64ByPairTuple::new(
                pair_average_price_repository,
                "coin_average_price".to_string(),
                None,
            ),
            capitalization: HashMap::new(),
            last_capitalization_refresh: MIN_DATETIME,
        };

        let worker = Arc::new(Mutex::new(worker));
        worker.lock().unwrap().set_arc(Arc::clone(&worker));

        worker
    }

    fn set_arc(&mut self, arc: Arc<Mutex<Self>>) {
        self.arc = Some(arc);
    }

    pub fn add_ws_channel(
        &mut self,
        conn_id: String,
        channel: WsChannelResponseSender,
    ) -> Result<(), Vec<String>> {
        match &channel.request {
            WsChannelRequest::CoinAveragePrice { .. } => {
                // Worker's channel

                self.pair_average_price
                    .ws_channels
                    .add_channel(conn_id, channel);

                Ok(())
            }
            WsChannelRequest::CoinExchangePrice { exchanges, .. }
            | WsChannelRequest::CoinExchangeVolume { exchanges, .. } => {
                // Market's channel

                // Search for unsupported exchanges
                let errors: Vec<String> = exchanges
                    .iter()
                    .filter(|&v| !self.markets.contains_key(v))
                    .map(|v| format!("Unsupported exchange: {}", v))
                    .collect();

                if !errors.is_empty() {
                    Err(errors)
                } else {
                    let key = (conn_id.clone(), channel.request.get_method());
                    self.remove_ws_channel(&key);
                    self.market_names_by_ws_channel_key
                        .insert(key, exchanges.clone());
                    let pairs: Vec<(&str, &str)> = channel
                        .request
                        .get_coins()
                        .iter()
                        .map(|v| (v.as_str(), "USD"))
                        .collect();

                    for exchange in exchanges {
                        for pair_tuple in &pairs {
                            let pair_string = self
                                .markets
                                .get_mut(exchange)
                                .unwrap()
                                .lock()
                                .unwrap()
                                .make_pair(*pair_tuple);

                            let mut market =
                                self.markets.get_mut(exchange).unwrap().lock().unwrap();

                            // There we have only supported exchanges, thus we can call `unwrap`
                            // But coins are not validated, so we need to check if pair exists
                            let exchange_pair_info = market
                                .get_spine_mut()
                                .get_exchange_pairs_mut()
                                .get_mut(&pair_string);

                            if let Some(exchange_pair_info) = exchange_pair_info {
                                exchange_pair_info
                                    .last_trade_price
                                    .ws_channels
                                    .add_channel(conn_id.clone(), channel.clone());

                                exchange_pair_info
                                    .total_volume
                                    .ws_channels
                                    .add_channel(conn_id.clone(), channel.clone());
                            }
                        }
                    }

                    Ok(())
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn remove_ws_channel(&mut self, key: &(String, String)) {
        if let Some(market_names) = self.market_names_by_ws_channel_key.get(key) {
            // Market's channel

            for market_name in market_names {
                let mut market = self.markets.get_mut(market_name).unwrap().lock().unwrap();

                let exchange_pair_infos =
                    market.get_spine_mut().get_exchange_pairs_mut().values_mut();

                for exchange_pair_info in exchange_pair_infos {
                    exchange_pair_info
                        .last_trade_price
                        .ws_channels
                        .remove_channel(key);

                    exchange_pair_info
                        .total_volume
                        .ws_channels
                        .remove_channel(key);
                }
            }
        } else {
            // Worker's channel

            self.pair_average_price.ws_channels.remove_channel(key);
        }
    }

    // TODO: Implement
    pub fn recalculate_total_volume(&self, _currency: String) {}

    pub fn recalculate_pair_average_price(&mut self, pair: (String, String), new_price: f64) {
        let old_avg = self
            .pair_average_price
            .get_value(&pair)
            .unwrap_or(new_price);

        let new_avg = (new_price + old_avg) / 2.0;

        info!("new {}-{} average trade price: {}", pair.0, pair.1, new_avg);

        self.pair_average_price.set_new_value(pair, new_avg);
    }

    fn refresh_capitalization_thread(&self) {
        let worker = Arc::clone(self.arc.as_ref().unwrap());
        let thread_name = "fn: refresh_capitalization".to_string();
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || loop {
                if is_graceful_shutdown(&worker) {
                    return;
                }

                if Self::refresh_capitalization(Arc::clone(&worker)).is_some() {
                    // if success
                    break;
                } else {
                    // if error
                    thread::sleep(time::Duration::from_millis(10000));
                }
            })
            .unwrap();
        self.tx.send(thread).unwrap();
    }

    fn refresh_capitalization(worker: Arc<Mutex<Self>>) -> Option<()> {
        let response = Client::new()
            .get("https://pro-api.coinmarketcap.com/v1/cryptocurrency/listings/latest")
            .header("Accepts", "application/json")
            .header("X-CMC_PRO_API_KEY", "388b6445-3e65-4b86-913e-f0534596068b")
            .multipart(
                Form::new()
                    .part("start", Part::text("1"))
                    .part("limit", Part::text("10"))
                    .part("convert", Part::text("USD")),
            )
            .send();

        match response {
            Ok(response) => match response.text() {
                Ok(response) => match serde_json::from_str(&response) {
                    Ok(json) => {
                        // This line is needed for type annotation
                        let json: serde_json::Value = json;

                        let json_object = json.as_object()?;
                        let coins = json_object.get("data")?.as_array()?;

                        for coin in coins.iter().map(|j| j.as_object().unwrap()) {
                            let mut curr = coin.get("symbol")?.as_str()?;
                            if curr == "MIOTA" {
                                curr = "IOT";
                            }

                            let total_supply = coin.get("total_supply")?.as_f64()?;

                            worker
                                .lock()
                                .unwrap()
                                .capitalization
                                .insert(curr.to_string(), total_supply);
                        }

                        worker.lock().unwrap().last_capitalization_refresh = Utc::now();
                        return Some(());
                    }
                    Err(e) => {
                        error!("Coinmarketcap.com: Failed to parse json. Error: {}", e)
                    }
                },
                Err(e) => error!(
                    "Coinmarketcap.com: Failed to get message text. Error: {}",
                    e
                ),
            },
            Err(e) => error!("Coinmarketcap.com: Failed to connect. Error: {}", e),
        }

        None
    }

    fn get_last_capitalization_refresh(&self) -> DateTime<Utc> {
        self.last_capitalization_refresh
    }

    fn is_graceful_shutdown(&self) -> bool {
        *self.graceful_shutdown.lock().unwrap()
    }

    fn configure(
        &mut self,
        markets: Vec<&str>,
        exchange_pairs: Vec<ExchangePair>,
        channels: Vec<MarketChannels>,
        rest_timeout_sec: u64,
        repositories: Option<RepositoriesByMarketName>,
    ) {
        let mut repositories = repositories.unwrap_or(HashMap::new());

        for market_name in markets {
            let worker_2 = Arc::clone(self.arc.as_ref().unwrap());
            let market_spine = MarketSpine::new(
                worker_2,
                self.tx.clone(),
                rest_timeout_sec,
                market_name.to_string(),
                channels.clone(),
                Arc::clone(&self.graceful_shutdown),
            );
            let market = market_factory(
                market_spine,
                exchange_pairs.clone(),
                repositories.remove(market_name),
            );

            self.markets.insert(market_name.to_string(), market);
        }
    }

    fn start_ws(
        &self,
        ws: bool,
        ws_addr: String,
        ws_answer_timeout_ms: u64,
        graceful_shutdown: Arc<Mutex<bool>>,
    ) {
        if ws {
            let worker = Arc::clone(self.arc.as_ref().unwrap());

            let thread_name = "fn: start_ws".to_string();
            let thread = thread::Builder::new()
                .name(thread_name)
                .spawn(move || {
                    let ws_server = WsServer {
                        worker,
                        ws_addr,
                        ws_answer_timeout_ms,
                        graceful_shutdown,
                    };
                    ws_server.start();
                })
                .unwrap();
            self.tx.send(thread).unwrap();
        }
    }

    pub fn start(&mut self, config: ConfigScheme, repositories: Option<RepositoriesByMarketName>) {
        let ConfigScheme { market, service } = config;
        let MarketConfig {
            markets,
            exchange_pairs,
            channels,
        } = market;
        let ServiceConfig {
            rest_timeout_sec,
            ws,
            ws_addr,
            ws_answer_timeout_ms,
            historical: _,
            storage: _,
        } = service;

        let markets = markets.iter().map(|v| v.as_ref()).collect();

        self.configure(
            markets,
            exchange_pairs,
            channels,
            rest_timeout_sec,
            repositories,
        );
        self.start_ws(
            ws,
            ws_addr,
            ws_answer_timeout_ms,
            self.graceful_shutdown.clone(),
        );

        self.refresh_capitalization_thread();

        for market in self.markets.values().cloned() {
            if self.is_graceful_shutdown() {
                return;
            }

            let thread_name = format!(
                "fn: perform, market: {}",
                market.lock().unwrap().get_spine().name,
            );
            let thread = thread::Builder::new()
                .name(thread_name)
                .spawn(move || {
                    market.lock().unwrap().perform();
                })
                .unwrap();
            self.tx.send(thread).unwrap();
        }
    }
}

#[cfg(test)]
pub mod test {
    use crate::config_scheme::config_scheme::ConfigScheme;
    use crate::config_scheme::market_config::MarketConfig;
    use crate::config_scheme::service_config::ServiceConfig;
    use crate::worker::market_helpers::conversion_type::ConversionType;
    use crate::worker::market_helpers::exchange_pair::ExchangePair;
    use crate::worker::market_helpers::market_channels::MarketChannels;
    use crate::worker::network_helpers::ws_server::ws_channels::test::check_subscriptions;
    use crate::worker::worker::Worker;
    use chrono::{Duration, Utc};
    use ntest::timeout;
    use serial_test::serial;
    use std::collections::HashMap;
    use std::sync::mpsc::{Receiver, Sender};
    use std::sync::{mpsc, Arc, Mutex};
    use std::thread;
    use std::thread::JoinHandle;
    use std::time;

    pub fn make_worker() -> (
        Arc<Mutex<Worker>>,
        Sender<JoinHandle<()>>,
        Receiver<JoinHandle<()>>,
    ) {
        let (tx, rx) = mpsc::channel();
        let graceful_shutdown = Arc::new(Mutex::new(false));
        let worker = Worker::new(tx.clone(), graceful_shutdown);

        (worker, tx, rx)
    }

    pub fn check_threads(mut thread_names: Vec<String>, rx: Receiver<JoinHandle<()>>) {
        let mut passed_thread_names = Vec::new();
        for received_thread in rx {
            let thread_name = received_thread.thread().name().unwrap().to_string();
            assert!(!passed_thread_names.contains(&thread_name));

            if let Some(index) = thread_names.iter().position(|r| r == &thread_name) {
                passed_thread_names.push(thread_names.swap_remove(index));
            }
            if thread_names.is_empty() {
                break;
            }
        }
    }

    #[test]
    fn test_new() {
        let (worker, _, _) = make_worker();

        assert!(worker.lock().unwrap().arc.is_some());
    }

    fn test_configure(markets: Vec<&str>, coins: Vec<&str>, channels: Vec<MarketChannels>) {
        let (worker, _, _) = make_worker();
        worker
            .lock()
            .unwrap()
            .configure(markets.clone(), coins.clone(), channels, 1);

        assert_eq!(markets.len(), worker.lock().unwrap().markets.len());

        for (market_name_key, market) in &worker.lock().unwrap().markets {
            let market_name = market.lock().unwrap().get_spine().name.clone();
            assert_eq!(market_name_key, &market_name);
            assert!(markets.contains(&market_name.as_str()));
        }
    }

    #[test]
    fn test_configure_with_default_params() {
        let config = MarketConfig::default();
        let markets = config.markets.iter().map(|v| v.as_ref()).collect();
        let coins = config.coins.iter().map(|v| v.as_ref()).collect();

        test_configure(markets, coins, config.channels);
    }

    #[test]
    fn test_configure_with_custom_params() {
        let markets = vec!["binance", "bitfinex"];
        let coins = vec!["ABC", "DEF", "GHI"];
        let channels = vec![MarketChannels::Ticker];

        test_configure(markets, coins, channels);
    }

    #[test]
    #[should_panic]
    fn test_configure_panic() {
        let config = MarketConfig::default();
        let markets = vec!["not_existing_market"];
        let coins = config.coins.iter().map(|v| v.as_ref()).collect();

        test_configure(markets, coins, config.channels);
    }

    #[test]
    fn test_recalculate_pair_average_price() {
        let (worker, _, _) = make_worker();

        let coins = ["BTC", "ETH"];
        let prices = [100.0, 200.0];

        for coin in coins {
            for new_price in prices {
                let pair = (coin.to_string(), "USD".to_string());
                let expected_curr_price = if new_price.eq(&100.0) {
                    100.0
                } else if new_price.eq(&200.0) {
                    150.0
                } else {
                    panic!("Test: Wrong price in loop.")
                };

                worker
                    .lock()
                    .unwrap()
                    .recalculate_pair_average_price(pair.clone(), new_price);

                let real_curr_price_field = worker
                    .lock()
                    .unwrap()
                    .pair_average_price
                    .get_value(&pair)
                    .unwrap();
                assert!(expected_curr_price.eq(&real_curr_price_field));
            }
        }
    }

    fn inner_test_refresh_capitalization(worker: Arc<Mutex<Worker>>) {
        let now = Utc::now();
        let last_capitalization_refresh = worker.lock().unwrap().get_last_capitalization_refresh();
        assert!(now - last_capitalization_refresh <= Duration::milliseconds(5000));
    }

    // #[test]
    /// TODO: Rework (make test independent of coinmarketcap.com)
    fn test_refresh_capitalization() {
        let (worker, _, _) = make_worker();
        let result = Worker::refresh_capitalization(Arc::clone(&worker));

        assert!(result.is_some());

        inner_test_refresh_capitalization(worker);
    }

    fn inner_test_make_exchange_pairs(
        coins: Vec<&str>,
        fiats: Option<Vec<&str>>,
        expected_exchange_pairs: Vec<ExchangePair>,
    ) {
        let real_exchange_pairs = Worker::make_exchange_pairs(coins, fiats);

        assert_eq!(expected_exchange_pairs.len(), real_exchange_pairs.len());
        for (i, expected_exchange_pair) in expected_exchange_pairs.into_iter().enumerate() {
            assert_eq!(expected_exchange_pair, real_exchange_pairs[i]);
        }
    }

    #[test]
    fn test_make_exchange_pairs_with_default_params() {
        let expected_exchange_pairs = vec![
            ExchangePair {
                pair: ("BTC".to_string(), "USD".to_string()),
                conversion: ConversionType::None,
            },
            ExchangePair {
                pair: ("ETH".to_string(), "USD".to_string()),
                conversion: ConversionType::None,
            },
        ];

        let config = MarketConfig::default();
        let coins = config.coins.iter().map(|v| v.as_ref()).collect();

        inner_test_make_exchange_pairs(coins, None, expected_exchange_pairs);
    }

    #[test]
    fn test_make_exchange_pairs_with_custom_params() {
        let expected_exchange_pairs = vec![
            ExchangePair {
                pair: ("ABC".to_string(), "GHI".to_string()),
                conversion: ConversionType::None,
            },
            ExchangePair {
                pair: ("ABC".to_string(), "JKL".to_string()),
                conversion: ConversionType::None,
            },
            ExchangePair {
                pair: ("DEF".to_string(), "GHI".to_string()),
                conversion: ConversionType::None,
            },
            ExchangePair {
                pair: ("DEF".to_string(), "JKL".to_string()),
                conversion: ConversionType::None,
            },
        ];

        let coins = vec!["ABC", "DEF"];
        let fiats = Some(vec!["GHI", "JKL"]);

        inner_test_make_exchange_pairs(coins, fiats, expected_exchange_pairs);
    }

    /// TODO: Add tests for WsServer
    fn inner_test_start(markets: Vec<String>, coins: Vec<String>, channels: Vec<MarketChannels>) {
        // To prevent DDoS attack on exchanges
        thread::sleep(time::Duration::from_millis(3000));

        let (worker, _, rx) = make_worker();

        let mut thread_names = Vec::new();
        thread_names.push("fn: refresh_capitalization".to_string());
        for market in &markets {
            let thread_name = format!("fn: perform, market: {}", market);
            thread_names.push(thread_name);
        }

        let market = MarketConfig {
            markets,
            coins,
            channels,
        };
        let config = ConfigScheme {
            market,
            service: ServiceConfig::default(),
        };

        worker.lock().unwrap().start(config);
        check_threads(thread_names, rx);
    }

    /// `serial` and `timeout` are incompatible
    #[test]
    // #[serial]
    #[timeout(5000)]
    fn test_start_with_default_params() {
        let config = MarketConfig::default();

        inner_test_start(config.markets, config.coins, config.channels);
    }

    /// `serial` and `timeout` are incompatible
    #[test]
    // #[serial]
    #[timeout(5000)]
    fn test_start_with_custom_params() {
        let markets = vec!["binance".to_string(), "bitfinex".to_string()];
        let coins = vec!["ABC".to_string(), "DEF".to_string()];
        let channels = vec![MarketChannels::Ticker];

        inner_test_start(markets, coins, channels);
    }

    #[test]
    #[serial]
    #[should_panic]
    fn test_start_panic() {
        let config = MarketConfig::default();
        let markets = vec!["not_existing_market".to_string()];

        inner_test_start(markets, config.coins, config.channels);
    }

    pub fn check_worker_subscriptions(
        worker: &Arc<Mutex<Worker>>,
        subscriptions: Vec<(String, String, Vec<String>)>,
    ) {
        check_subscriptions(
            &worker.lock().unwrap().pair_average_price.ws_channels,
            &subscriptions,
        );
    }

    pub fn check_market_subscriptions(
        worker: &Arc<Mutex<Worker>>,
        subscriptions: Vec<(String, String, Vec<String>, Vec<String>)>,
    ) {
        let mut subscriptions_new = HashMap::new();
        for (sub_id, method, coins, exchanges) in subscriptions {
            for exchange in exchanges {
                let new_sub = (sub_id.clone(), method.clone(), coins.clone());
                subscriptions_new
                    .entry(exchange)
                    .or_insert(Vec::new())
                    .push(new_sub);
            }
        }

        for (market_name, market) in worker.lock().unwrap().markets.clone() {
            if let Some(subscriptions) = subscriptions_new.get(&market_name) {
                check_subscriptions(
                    &market.lock().unwrap().get_spine().ws_channels,
                    subscriptions,
                );
            } else {
                check_subscriptions(&market.lock().unwrap().get_spine().ws_channels, &Vec::new());
            }
        }
    }
}
