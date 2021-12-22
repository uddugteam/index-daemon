use crate::worker::market_helpers::conversion_type::ConversionType;
use crate::worker::market_helpers::exchange_pair::ExchangePair;
use crate::worker::market_helpers::exchange_pair_info::ExchangePairInfo;
use crate::worker::market_helpers::market::Market;
use crate::worker::worker::Worker;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

pub const EPS: f64 = 0.00001;

pub struct MarketSpine {
    worker: Arc<Mutex<Worker>>,
    pub arc: Option<Arc<Mutex<dyn Market + Send>>>,
    pub tx: Sender<JoinHandle<()>>,
    pub name: String,
    mask_pairs: HashMap<String, String>,
    unmask_pairs: HashMap<String, String>,
    exchange_pairs: HashMap<String, ExchangePairInfo>,
    conversions: HashMap<String, ConversionType>,
    pairs: HashMap<String, (String, String)>,
}
impl MarketSpine {
    pub fn new(worker: Arc<Mutex<Worker>>, tx: Sender<JoinHandle<()>>, name: String) -> Self {
        Self {
            worker,
            arc: None,
            tx,
            name,
            mask_pairs: HashMap::new(),
            unmask_pairs: HashMap::new(),
            exchange_pairs: HashMap::new(),
            conversions: HashMap::new(),
            pairs: HashMap::new(),
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

    // TODO: Implement
    /// Cannot be implemented, because it depends on https://api.icex.ch/api/coins/
    /// which is not working
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
                self.exchange_pairs
                    .get_mut(pair)
                    .unwrap()
                    .set_total_volume(value.abs());

                self.update_market_pair(pair, "totalValues", false);

                let pair_tuple = self.pairs.get(pair).unwrap();
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
                worker.lock().unwrap().recalculate_total_volume(currency);
            })
            .unwrap();
        self.tx.send(thread).unwrap();
    }

    fn recalculate_pair_average_trade_price(&self, pair: (String, String), value: f64) {
        let worker = Arc::clone(&self.worker);

        let thread_name = format!(
            "fn: recalculate_pair_average_trade_price, market: {}, pair: ({},{})",
            self.name, pair.0, pair.1,
        );
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                worker
                    .lock()
                    .unwrap()
                    .recalculate_pair_average_trade_price(pair, value);
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

        self.exchange_pairs
            .get_mut(pair)
            .unwrap()
            .set_last_trade_price(value);

        let pair_tuple = self.pairs.get(pair).unwrap().clone();
        self.recalculate_pair_average_trade_price(pair_tuple, value);

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
