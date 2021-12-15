use crate::worker::market_helpers::conversion_type::ConversionType;
use crate::worker::market_helpers::exchange_pair::ExchangePair;
use crate::worker::market_helpers::exchange_pair_info::ExchangePairInfo;
use std::collections::HashMap;
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
    exchange_pair_info: HashMap<(String, String), Vec<Arc<Mutex<ExchangePairInfo>>>>,
}

impl Worker {
    pub fn new(tx: Sender<JoinHandle<()>>) -> Arc<Mutex<Self>> {
        let worker = Worker {
            arc: None,
            tx,
            markets: Vec::new(),
            exchange_pair_info: HashMap::new(),
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
    pub fn recalculate_total_volume(&self, _currency: String) {
        // println!("called Worker::recalculate_total_volume()");
    }

    fn make_exchange_pairs(coins: Vec<&str>, fiats: Vec<&str>) -> Vec<ExchangePair> {
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

            for exchange_pair_info in market
                .lock()
                .unwrap()
                .get_spine()
                .get_exchange_pairs()
                .values()
            {
                let pair = exchange_pair_info.lock().unwrap().get_pair().clone();
                self.exchange_pair_info
                    .entry(pair)
                    .or_insert(vec![Arc::clone(exchange_pair_info)])
                    .push(Arc::clone(exchange_pair_info));
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
