use crate::repository::pair_average_trade_price_cache::PairAverageTradePriceCache;
use crate::repository::repository::Repository;
use crate::worker::defaults::{COINS, FIATS, MARKETS};
use crate::worker::market_helpers::conversion_type::ConversionType;
use crate::worker::market_helpers::exchange_pair::ExchangePair;
use chrono::{DateTime, Utc, MIN_DATETIME};
use reqwest::blocking::multipart::{Form, Part};
use reqwest::blocking::Client;
use rustc_serialize::json::Json;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time;

use crate::worker::market_helpers::market::{market_factory, Market};
use crate::worker::market_helpers::market_spine::MarketSpine;
use crate::worker::network_helpers::ws_server::WsServer;

pub struct Worker {
    arc: Option<Arc<Mutex<Self>>>,
    tx: Sender<JoinHandle<()>>,
    graceful_shutdown: Arc<Mutex<bool>>,
    markets: Vec<Arc<Mutex<dyn Market + Send>>>,
    pair_average_trade_price: HashMap<(String, String), f64>,
    pair_average_trade_price_repository: Arc<Mutex<dyn Repository<(String, String), f64> + Send>>,
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
    ) -> Arc<Mutex<Self>> {
        let pair_average_trade_price_repository =
            Arc::new(Mutex::new(PairAverageTradePriceCache::new()));

        let worker = Worker {
            arc: None,
            tx,
            graceful_shutdown,
            markets: Vec::new(),
            pair_average_trade_price: HashMap::new(),
            pair_average_trade_price_repository,
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

    fn set_pair_average_trade_price(&mut self, pair: (String, String), value: f64) {
        self.pair_average_trade_price.insert(pair.clone(), value);

        self.pair_average_trade_price_repository
            .lock()
            .unwrap()
            .insert(pair, value);
    }

    // TODO: Implement
    pub fn recalculate_total_volume(&self, _currency: String) {}

    pub fn recalculate_pair_average_trade_price(&mut self, pair: (String, String), new_price: f64) {
        let old_avg = *self
            .pair_average_trade_price
            .entry(pair.clone())
            .or_insert(new_price);

        let new_avg = (new_price + old_avg) / 2.0;

        info!("new {}-{} average trade price: {}", pair.0, pair.1, new_avg);

        self.set_pair_average_trade_price(pair, new_avg);
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
                Ok(response) => match Json::from_str(&response) {
                    Ok(json) => {
                        let json_object = json.as_object().unwrap();
                        let coins = json_object.get("data").unwrap().as_array().unwrap();

                        for coin in coins.iter().map(|j| j.as_object().unwrap()) {
                            let mut curr = coin.get("symbol").unwrap().as_string().unwrap();
                            if curr == "MIOTA" {
                                curr = "IOT";
                            }

                            let total_supply = coin.get("total_supply").unwrap().as_f64().unwrap();

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

    pub fn make_exchange_pairs(
        coins: Option<Vec<&str>>,
        fiats: Option<Vec<&str>>,
    ) -> Vec<ExchangePair> {
        let mut exchange_pairs = Vec::new();

        let coins = coins.unwrap_or(COINS.to_vec());
        let fiats = fiats.unwrap_or(FIATS.to_vec());

        for coin in coins {
            for fiat in &fiats {
                exchange_pairs.push(ExchangePair {
                    pair: (coin.to_string(), fiat.to_string()),
                    conversion: ConversionType::None,
                });
            }
        }

        exchange_pairs
    }

    fn is_graceful_shutdown(&self) -> bool {
        *self.graceful_shutdown.lock().unwrap()
    }

    fn configure(
        &mut self,
        markets: Option<Vec<&str>>,
        coins: Option<Vec<&str>>,
        channels: Option<Vec<&str>>,
        rest_timeout_sec: u64,
    ) {
        let market_names = markets.unwrap_or(MARKETS.to_vec());
        let exchange_pairs = Self::make_exchange_pairs(coins, None);
        let channels = channels.map(|v| v.into_iter().map(|v| v.parse().unwrap()).collect());

        for market_name in market_names {
            let worker_2 = Arc::clone(self.arc.as_ref().unwrap());
            let market_spine = MarketSpine::new(
                worker_2,
                self.tx.clone(),
                rest_timeout_sec,
                market_name.to_string(),
                channels.clone(),
                Arc::clone(&self.graceful_shutdown),
            );
            let market = market_factory(market_spine, exchange_pairs.clone());

            self.markets.push(market);
        }
    }

    fn start_ws(
        &self,
        ws: bool,
        ws_host: String,
        ws_port: String,
        ws_answer_timeout_sec: u64,
        graceful_shutdown: Arc<Mutex<bool>>,
    ) {
        if ws {
            let thread_name = "fn: start_ws".to_string();
            let thread = thread::Builder::new()
                .name(thread_name)
                .spawn(move || {
                    let ws_server = WsServer {
                        ws_host,
                        ws_port,
                        ws_answer_timeout_sec,
                        graceful_shutdown,
                    };
                    ws_server.start();
                })
                .unwrap();
            self.tx.send(thread).unwrap();
        }
    }

    pub fn start(
        &mut self,
        markets: Option<Vec<String>>,
        coins: Option<Vec<String>>,
        channels: Option<Vec<String>>,
        rest_timeout_sec: u64,
        ws: bool,
        ws_host: String,
        ws_port: String,
        ws_answer_timeout_sec: u64,
    ) {
        let markets = markets
            .as_ref()
            .map(|v| v.iter().map(|v| v.as_str()).collect());
        let coins = coins
            .as_ref()
            .map(|v| v.iter().map(|v| v.as_str()).collect());
        let channels = channels
            .as_ref()
            .map(|v| v.iter().map(|v| v.as_str()).collect());

        self.configure(markets, coins, channels, rest_timeout_sec);
        self.start_ws(
            ws,
            ws_host,
            ws_port,
            ws_answer_timeout_sec,
            self.graceful_shutdown.clone(),
        );

        self.refresh_capitalization_thread();

        for market in self.markets.iter().cloned() {
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
    use crate::worker::defaults::MARKETS;
    use crate::worker::market_helpers::conversion_type::ConversionType;
    use crate::worker::market_helpers::exchange_pair::ExchangePair;
    use crate::worker::worker::Worker;
    use chrono::{Duration, Utc};
    use ntest::timeout;
    use std::sync::mpsc::{Receiver, Sender};
    use std::sync::{mpsc, Arc, Mutex};
    use std::thread::JoinHandle;

    pub fn make_worker() -> (
        Arc<Mutex<Worker>>,
        Sender<JoinHandle<()>>,
        Receiver<JoinHandle<()>>,
    ) {
        let (tx, rx) = mpsc::channel();
        let worker = Worker::new(tx.clone());

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

    fn test_configure(
        markets: Option<Vec<&str>>,
        coins: Option<Vec<&str>>,
        channels: Option<Vec<&str>>,
    ) {
        let (worker, _, _) = make_worker();
        worker
            .lock()
            .unwrap()
            .configure(markets.clone(), coins.clone(), channels.clone(), None);

        let markets = markets.unwrap_or(MARKETS.to_vec());
        assert_eq!(markets.len(), worker.lock().unwrap().markets.len());

        for (i, market) in worker.lock().unwrap().markets.iter().enumerate() {
            let market_name = market.lock().unwrap().get_spine().name.clone();
            assert_eq!(market_name, markets[i]);
        }
    }

    #[test]
    fn test_configure_with_default_params() {
        test_configure(None, None, None);
    }

    #[test]
    fn test_configure_with_custom_params() {
        let markets = vec!["binance", "bitfinex"];
        let coins = vec!["ABC", "DEF", "GHI"];
        let channels = vec!["ticker"];

        test_configure(Some(markets), Some(coins), Some(channels));
    }

    #[test]
    #[should_panic]
    fn test_configure_panic() {
        let markets = vec!["not_existing_market"];

        test_configure(Some(markets), None, None);
    }

    #[test]
    #[should_panic]
    fn test_configure_panic_2() {
        let channels = vec!["not_existing_channel"];

        test_configure(None, None, Some(channels));
    }

    #[test]
    fn test_recalculate_pair_average_trade_price() {
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
                    .recalculate_pair_average_trade_price(pair.clone(), new_price);

                let real_curr_price_field = *worker
                    .lock()
                    .unwrap()
                    .pair_average_trade_price
                    .get(&pair)
                    .unwrap();
                assert!(expected_curr_price.eq(&real_curr_price_field));

                let real_curr_price_repo = worker
                    .lock()
                    .unwrap()
                    .pair_average_trade_price_repository
                    .lock()
                    .unwrap()
                    .read(pair)
                    .unwrap();
                assert!(expected_curr_price.eq(&real_curr_price_repo));
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
        coins: Option<Vec<&str>>,
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

        inner_test_make_exchange_pairs(None, None, expected_exchange_pairs);
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

        let coins = Some(vec!["ABC", "DEF"]);
        let fiats = Some(vec!["GHI", "JKL"]);

        inner_test_make_exchange_pairs(coins, fiats, expected_exchange_pairs);
    }

    fn inner_test_start(
        markets: Option<Vec<String>>,
        coins: Option<Vec<String>>,
        channels: Option<Vec<String>>,
    ) {
        let (worker, _, rx) = make_worker();

        let markets_2 = markets
            .as_ref()
            .map(|v| v.iter().map(|v| v.as_str()).collect())
            .unwrap_or(MARKETS.to_vec());

        let mut thread_names = Vec::new();
        thread_names.push("fn: refresh_capitalization".to_string());
        for market in markets_2 {
            let thread_name = format!("fn: perform, market: {}", market);
            thread_names.push(thread_name);
        }

        worker.lock().unwrap().start(markets, coins, channels, None);
        check_threads(thread_names, rx);
    }

    #[test]
    #[timeout(2000)]
    fn test_start_with_default_params() {
        inner_test_start(None, None, None);
    }

    #[test]
    #[timeout(2000)]
    fn test_start_with_custom_params() {
        let markets = Some(vec!["binance".to_string(), "bitfinex".to_string()]);
        let coins = Some(vec!["ABC".to_string(), "DEF".to_string()]);
        let channels = Some(vec!["ticker".to_string()]);

        inner_test_start(markets, coins, channels);
    }

    #[test]
    #[should_panic]
    fn test_start_panic() {
        let markets = Some(vec!["not_existing_market".to_string()]);

        inner_test_start(markets, None, None);
    }

    #[test]
    #[should_panic]
    fn test_start_panic_2() {
        let channels = Some(vec!["not_existing_channel".to_string()]);

        inner_test_start(None, None, channels);
    }
}
