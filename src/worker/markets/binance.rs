use std::thread;
use std::thread::JoinHandle;
use std::time;

use crate::worker::market_helpers::market::{Market, MarketSpine};

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
    fn subscribe_ticker(&self, pair: &str) {}
}

impl Market for Binance {
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

    // TODO: Rework function to allow spawn inside it multiple threads, that borrow `self`
    fn update(&mut self) {
        println!("called Binance::update()");

        self.spine.socket_enabled = true;

        if !self.spine.socket_enabled {
            for exchange_pair in self.spine.get_exchange_pairs() {
                self.update_ticker(exchange_pair.0);
                self.update_trades(exchange_pair.0);
                self.update_depth(exchange_pair.0);
            }
        } else {
            println!("Binance: Delay: {}", self.spine.delay);

            let keys: Vec<String> = self
                .spine
                .get_exchange_pairs()
                .keys()
                .into_iter()
                .cloned()
                .collect();

            let mut threads: Vec<JoinHandle<()>> = Vec::new();

            for exchange_pair in self.spine.get_exchange_pairs().clone() {
                let thread = thread::spawn(|| loop {
                    self.subscribe_ticker(&exchange_pair.0);
                    thread::sleep(time::Duration::from_millis(self.spine.delay.into()));
                });

                threads.push(thread);
            }

            while !threads.is_empty() {
                let thread = threads.swap_remove(0);
                thread.join().unwrap();
            }
        }
    }
}
