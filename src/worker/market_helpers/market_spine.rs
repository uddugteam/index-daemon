use crate::worker::market_helpers::conversion_type::ConversionType;
use crate::worker::market_helpers::exchange_pair::ExchangePair;
use crate::worker::market_helpers::exchange_pair_info::ExchangePairInfo;
use crate::worker::market_helpers::market::Market;
use crate::worker::worker::Worker;
use chrono::{DateTime, Utc, MIN_DATETIME};
use reqwest::blocking::multipart::{Form, Part};
use reqwest::blocking::Client;
use rustc_serialize::json::Json;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

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
    pub socket_enabled: bool,
    capitalization: HashMap<String, f64>,
    last_capitalization_refresh: DateTime<Utc>,
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
            socket_enabled: false,
            capitalization: HashMap::new(),
            last_capitalization_refresh: MIN_DATETIME,
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

    pub fn refresh_capitalization(&mut self) {
        // println!("called MarketSpine::refresh_capitalization()");

        let response_text = Client::new()
            .get("https://pro-api.coinmarketcap.com/v1/cryptocurrency/listings/latest")
            .header("Accepts", "application/json")
            .header("X-CMC_PRO_API_KEY", "388b6445-3e65-4b86-913e-f0534596068b")
            .multipart(
                Form::new()
                    .part("start", Part::text("1"))
                    .part("limit", Part::text("10"))
                    .part("convert", Part::text("USD")),
            )
            .send()
            .unwrap()
            .text()
            .unwrap();

        if let Ok(json) = Json::from_str(&response_text) {
            let json_object = json.as_object().unwrap();
            let coins = json_object.get("data").unwrap().as_array().unwrap();

            for coin in coins.iter().map(|j| j.as_object().unwrap()) {
                let mut curr = coin.get("symbol").unwrap().as_string().unwrap();
                if curr == "MIOTA" {
                    curr = "IOT";
                }

                let total_supply = coin.get("total_supply").unwrap().as_f64().unwrap();

                self.capitalization.insert(curr.to_string(), total_supply);
            }

            self.last_capitalization_refresh = Utc::now();
        }
    }

    // TODO: Implement
    /// Cannot be implemented, because it depends on https://api.icex.ch/api/coins/
    /// which is no working
    /// Temporary solution: hardcode two coins: BTC and ETH
    pub fn get_conversion_coef(&mut self, currency: &str, conversion: ConversionType) -> f64 {
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

                // Never lock Worker inside methods of Market
                self.recalculate_total_volume(self.pairs.get(pair).unwrap().0.clone());
            }
        }
    }

    fn recalculate_total_volume(&self, currency: String) {
        // println!("called MarketSpine::recalculate_total_volume()");

        let worker = Arc::clone(&self.worker);

        let thread = thread::spawn(move || {
            worker.lock().unwrap().recalculate_total_volume(currency);
        });
        self.tx.send(thread).unwrap();
    }

    // TODO: Implement
    pub fn update_market_pair(&mut self, pair: &str, scope: &str, price_changed: bool) {}

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
        if !old_value.eq(&-1.0) {
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
