use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time;

use crate::worker::market_helpers::action::Action;
use crate::worker::market_helpers::market::{Market, MarketSpine};
use crate::worker::market_helpers::market_name::MarketName;

pub struct Binance {
    pub spine: MarketSpine,
}

impl Binance {
    // TODO: Implement
    fn update_ticker(&self, pair: &str) {}

    // TODO: Implement
    fn update_trades(&self, pair: &str) {}

    // TODO: Implement
    fn update_depth(&self, pair: &str) {}

    // TODO: Implement
    pub fn subscribe_ticker(market: Arc<Mutex<Box<RefCell<dyn Market + Send>>>>, pair: String) {
        println!("called Binance::subscribe_ticker()");
    }

    // TODO: Implement
    pub fn subscribe_trades(market: Arc<Mutex<Box<RefCell<dyn Market + Send>>>>, pair: String) {
        println!("called Binance::subscribe_trades()");
    }

    // TODO: Implement
    pub fn subscribe_depth(market: Arc<Mutex<Box<RefCell<dyn Market + Send>>>>, pair: String) {
        println!("called Binance::subscribe_depth()");
    }

    pub fn subscribe_threads(
        market: Arc<Mutex<Box<RefCell<dyn Market + Send>>>>,
        pair: String,
        delay: u64,
    ) -> Vec<JoinHandle<()>> {
        println!("called Binance::subscribe_threads()");

        let mut threads: Vec<JoinHandle<()>> = Vec::new();

        let market_2 = Arc::clone(&market);
        let pair_2 = pair.clone();
        let thread = thread::spawn(move || {
            let market_3 = Arc::clone(&market_2);
            let pair_3 = pair_2.clone();
            loop {
                Self::subscribe_ticker(Arc::clone(&market_3), pair_3.clone());
                thread::sleep(time::Duration::from_millis(delay));
            }
        });
        threads.push(thread);

        let market_2 = Arc::clone(&market);
        let pair_2 = pair.clone();
        let thread = thread::spawn(move || {
            let market_3 = Arc::clone(&market_2);
            let pair_3 = pair_2.clone();
            loop {
                Self::subscribe_trades(Arc::clone(&market_3), pair_3.clone());
                thread::sleep(time::Duration::from_millis(delay));
            }
        });
        threads.push(thread);

        let market_2 = Arc::clone(&market);
        let pair_2 = pair.clone();
        let thread = thread::spawn(move || {
            let market_3 = Arc::clone(&market_2);
            let pair_3 = pair_2.clone();
            loop {
                Self::subscribe_depth(Arc::clone(&market_3), pair_3.clone());
                thread::sleep(time::Duration::from_millis(delay));
            }
        });
        threads.push(thread);

        threads
    }
}

impl Market for Binance {
    fn get_name(&self) -> MarketName {
        MarketName::Binance
    }

    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn make_pair(&self, pair: (&str, &str)) -> String {
        (self.spine.get_masked_value(pair.0).to_string() + self.spine.get_masked_value(pair.1))
            .to_lowercase()
    }

    fn add_exchange_pair(&mut self, pair: (&str, &str), conversion: &str) {
        let pair_string = self.make_pair(pair);
        self.spine.add_exchange_pair(pair_string, pair, conversion);
    }

    fn update(&mut self) -> Vec<Action> {
        println!("called Binance::update()");

        let mut actions: Vec<Action> = Vec::new();

        self.spine.socket_enabled = true;

        if !self.spine.socket_enabled {
            for exchange_pair in self.spine.get_exchange_pairs() {
                self.update_ticker(exchange_pair.0);
                self.update_trades(exchange_pair.0);
                self.update_depth(exchange_pair.0);
            }
        } else {
            println!("Binance: Delay: {}", self.spine.delay);

            let delay = self.spine.delay.into();

            for exchange_pair in self.spine.get_exchange_pairs().clone() {
                actions.push(Action::SubscribeTickerTradesDepth {
                    pair: exchange_pair.0.to_string(),
                    delay,
                });
            }
        }

        actions
    }
}
