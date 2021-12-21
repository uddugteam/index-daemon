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

pub struct Worker {
    arc: Option<Arc<Mutex<Self>>>,
    tx: Sender<JoinHandle<()>>,
    markets: Vec<Arc<Mutex<dyn Market + Send>>>,
    pair_average_trade_price: HashMap<(String, String), f64>,
    pair_average_trade_price_repository: Arc<Mutex<dyn Repository<(String, String), f64> + Send>>,
    capitalization: HashMap<String, f64>,
    last_capitalization_refresh: DateTime<Utc>,
}

impl Worker {
    pub fn new(
        tx: Sender<JoinHandle<()>>,
        pair_average_trade_price_repository: Arc<
            Mutex<dyn Repository<(String, String), f64> + Send>,
        >,
    ) -> Arc<Mutex<Self>> {
        let worker = Worker {
            arc: None,
            tx,
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

    pub fn set_arc(&mut self, arc: Arc<Mutex<Self>>) {
        self.arc = Some(arc);
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

        self.pair_average_trade_price.insert(pair.clone(), new_avg);

        self.pair_average_trade_price_repository
            .lock()
            .unwrap()
            .insert(pair, new_avg);
    }

    pub fn refresh_capitalization(&mut self) {
        loop {
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

                                let total_supply =
                                    coin.get("total_supply").unwrap().as_f64().unwrap();

                                self.capitalization.insert(curr.to_string(), total_supply);
                            }

                            self.last_capitalization_refresh = Utc::now();
                            break;
                        }
                        Err(e) => error!("Coinmarketcap.com: Failed to parse json. Error: {}", e),
                    },
                    Err(e) => error!(
                        "Coinmarketcap.com: Failed to get message text. Error: {}",
                        e
                    ),
                },
                Err(e) => error!("Coinmarketcap.com: Failed to connect. Error: {}", e),
            }

            thread::sleep(time::Duration::from_millis(10000));
        }
    }

    fn get_last_capitalization_refresh(&self) -> DateTime<Utc> {
        self.last_capitalization_refresh
    }

    pub fn make_exchange_pairs(coins: Vec<&str>, fiats: Vec<&str>) -> Vec<ExchangePair> {
        let mut exchange_pairs = Vec::new();

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

    fn configure(&mut self, markets: Option<Vec<&str>>, coins: Option<Vec<&str>>) {
        let market_names = markets.unwrap_or(Vec::from(MARKETS));
        let fiats = Vec::from(FIATS);
        let coins = coins.unwrap_or(Vec::from(COINS));
        let exchange_pairs = Self::make_exchange_pairs(coins, fiats);

        for market_name in market_names {
            let worker_2 = Arc::clone(self.arc.as_ref().unwrap());
            let market_spine = MarketSpine::new(worker_2, self.tx.clone(), market_name.to_string());
            let market = market_factory(market_spine, exchange_pairs.clone());

            self.markets.push(market);
        }
    }

    pub fn start(&mut self, markets: Option<Vec<&str>>, coins: Option<Vec<&str>>) {
        self.configure(markets, coins);
        self.refresh_capitalization();

        for market in self.markets.iter().cloned() {
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
    use crate::repository::pair_average_trade_price::PairAverageTradePrice;
    use crate::worker::defaults::MARKETS;
    use crate::worker::worker::Worker;
    use chrono::{Duration, Utc};
    use ntest::timeout;
    use std::sync::mpsc::{Receiver, Sender};
    use std::sync::{mpsc, Arc, Mutex};
    use std::thread::JoinHandle;

    pub fn get_worker() -> (
        Arc<Mutex<Worker>>,
        Sender<JoinHandle<()>>,
        Receiver<JoinHandle<()>>,
    ) {
        let (tx, rx) = mpsc::channel();
        let pair_average_trade_price_repository = PairAverageTradePrice::new();
        let worker = Worker::new(
            tx.clone(),
            Arc::new(Mutex::new(pair_average_trade_price_repository)),
        );

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
        let (worker, _, _) = get_worker();

        assert!(worker.lock().unwrap().arc.is_some());
    }

    fn test_configure(markets: Option<Vec<&str>>, coins: Option<Vec<&str>>) {
        let (worker, _, _) = get_worker();
        worker
            .lock()
            .unwrap()
            .configure(markets.clone(), coins.clone());

        let markets = markets.unwrap_or(Vec::from(MARKETS));
        assert_eq!(markets.len(), worker.lock().unwrap().markets.len());

        for (i, market) in worker.lock().unwrap().markets.iter().enumerate() {
            let market_name = market.lock().unwrap().get_spine().name.clone();
            assert_eq!(market_name, markets[i]);
        }
    }

    #[test]
    fn test_configure_with_default_params() {
        test_configure(None, None);
    }

    #[test]
    fn test_configure_with_custom_params() {
        let markets = vec!["binance", "bitfinex"];
        let coins = vec!["ABC", "DEF", "GHI"];

        test_configure(Some(markets), Some(coins));
    }

    #[test]
    #[timeout(2000)]
    fn test_start() {
        let (worker, _, rx) = get_worker();

        let markets = Vec::from(MARKETS);
        let mut thread_names = Vec::new();
        for market in markets {
            let thread_name = format!("fn: perform, market: {}", market);
            thread_names.push(thread_name);
        }

        worker.lock().unwrap().start(None, None);
        check_threads(thread_names, rx);

        let now = Utc::now();
        let last_capitalization_refresh = worker.lock().unwrap().get_last_capitalization_refresh();
        assert!(now - last_capitalization_refresh <= Duration::milliseconds(5000));
    }
}
