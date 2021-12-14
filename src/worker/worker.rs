use crate::worker::market_helpers::conversion_type::ConversionType;
use crate::worker::market_helpers::exchange_pair::ExchangePair;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

use crate::worker::market_helpers::market::{market_factory, Market};
use crate::worker::market_helpers::market_spine::MarketSpine;

pub struct Worker {
    arc: Option<Arc<Mutex<Self>>>,
    tx: Sender<JoinHandle<()>>,
    markets: Vec<Arc<Mutex<dyn Market + Send>>>,
}

impl Worker {
    pub fn new(tx: Sender<JoinHandle<()>>) -> Arc<Mutex<Self>> {
        let worker = Worker {
            arc: None,
            tx,
            markets: Vec::new(),
        };

        let worker = Arc::new(Mutex::new(worker));
        worker.lock().unwrap().set_arc(Arc::clone(&worker));

        worker
    }

    pub fn set_arc(&mut self, arc: Arc<Mutex<Self>>) {
        self.arc = Some(arc);
    }

    // TODO: Implement
    /// Never lock Worker inside methods of Market
    pub fn recalculate_total_volume(&self, currency: String) {
        // println!("called Worker::recalculate_total_volume()");

        let mut volume_val: f64 = 0.0;
        let mut markets_count: i32 = 0;
        let mut buf: String = String::new();
        let mut success: bool = true;
        let mut fail_count: i32 = 0;

        for market in &self.markets {
            let market = market.lock().unwrap();

            for pair in market.get_spine().get_pairs() {
                if pair.1 .0 == currency {
                    markets_count += 1;
                    let volume: f64 = market.get_total_volume(&pair.1 .0, &pair.1 .1);

                    if volume.eq(&-1.0) {
                        fail_count += 1;
                        success = false;

                        // C++: loggingHelper->printLog("volume", 2, currency + " " + market->getName() + " " + pair.first);
                        // println!("{} {} {}", currency, market.get_spine().name, pair.0);
                    }

                    volume_val += volume;

                    buf = format!(
                        "{}: {}_{} {} ",
                        market.get_spine().name,
                        pair.1 .0,
                        pair.1 .1,
                        volume
                    );
                }
            }
        }

        if !success {
            // C++: loggingHelper->printLog("volume", 2, currency + " NOT ENOUGH VALUES " + std::to_string(markets_count - failCount) + " OUT OF " + std::to_string(markets_count));
            println!(
                "{} NOT ENOUGH VALUES {} OUT OF {}",
                currency,
                markets_count - fail_count,
                markets_count
            );

            return;
        }

        // C++: loggingHelper->printLog("volume", 1, currency + ": " + std::to_string(volume_val));
        // C++: loggingHelper->printLog("volume", 3, currency + ": " + buf);
        // println!("{}: {}", currency, volume_val);
        // println!("{}: {}", currency, buf);

        // C++: code, associated with Redis
        // C++: code, associated with candles
    }

    fn make_exchange_pairs(coins: Vec<&str>, fiats: Vec<&str>) -> Vec<ExchangePair> {
        let mut exchange_pairs = Vec::new();

        for (key_1, coin_1) in coins.iter().enumerate() {
            for key_2 in key_1 + 1..coins.len() {
                let coin_2 = coins[key_2];

                if *coin_1 != coin_2 {
                    exchange_pairs.push(ExchangePair {
                        pair: (coin_1.to_string(), coin_2.to_string()),
                        conversion: ConversionType::Crypto,
                    });
                }
            }
        }

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
        let market_names = markets.unwrap_or(vec!["binance", "bitfinex", "coinbase"]);

        let fiats = vec!["USD"];
        let coins = coins.unwrap_or(vec!["BTC", "ETH"]);

        let exchange_pairs = Self::make_exchange_pairs(coins, fiats);

        for market_name in market_names {
            let worker_2 = Arc::clone(self.arc.as_ref().unwrap());

            let market_spine = MarketSpine::new(worker_2, self.tx.clone(), market_name.to_string());

            let market = market_factory(market_spine);

            for exchange_pair in &exchange_pairs {
                market
                    .lock()
                    .unwrap()
                    .add_exchange_pair(exchange_pair.clone());
            }

            self.markets.push(market);
        }
    }

    pub fn start(&mut self, markets: Option<Vec<&str>>, coins: Option<Vec<&str>>) {
        self.configure(markets, coins);

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
