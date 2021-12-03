use crate::worker::market_helpers::exchange_pair_info::ExchangePairInfo;
use crate::worker::market_helpers::status::Status;
use crate::worker::worker::Worker;
use chrono::{DateTime, Utc, MIN_DATETIME};
use reqwest::blocking::multipart::{Form, Part};
use reqwest::blocking::Client;
use rustc_serialize::json::Json;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

const EPS: f64 = 0.00001;

pub struct MarketSpine {
    worker: Arc<Mutex<Worker>>,
    pub tx: Sender<JoinHandle<()>>,
    pub status: Status,
    pub name: String,
    api_url: String,
    error_message: String,
    pub delay: u32,
    mask_pairs: HashMap<String, String>,
    unmask_pairs: HashMap<String, String>,
    exchange_pairs: HashMap<String, ExchangePairInfo>,
    conversions: HashMap<String, String>,
    pairs: HashMap<String, (String, String)>,
    update_ticker: bool,
    update_last_trade: bool,
    update_depth: bool,
    fiat_refresh_time: u64,
    pub socket_enabled: bool,
    capitalization: HashMap<String, f64>,
    last_capitalization_refresh: DateTime<Utc>,
}
impl MarketSpine {
    pub fn new(
        worker: Arc<Mutex<Worker>>,
        tx: Sender<JoinHandle<()>>,
        status: Status,
        name: String,
        api_url: String,
        error_message: String,
        delay: u32,
        update_ticker: bool,
        update_last_trade: bool,
        update_depth: bool,
        fiat_refresh_time: u64,
    ) -> Self {
        Self {
            worker,
            tx,
            status,
            name,
            api_url,
            error_message,
            delay,
            mask_pairs: HashMap::new(),
            unmask_pairs: HashMap::new(),
            exchange_pairs: HashMap::new(),
            conversions: HashMap::new(),
            pairs: HashMap::new(),
            update_ticker,
            update_last_trade,
            update_depth,
            fiat_refresh_time,
            socket_enabled: false,
            capitalization: HashMap::new(),
            last_capitalization_refresh: MIN_DATETIME,
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

    pub fn add_exchange_pair(&mut self, pair_string: String, pair: (&str, &str), conversion: &str) {
        self.exchange_pairs
            .insert(pair_string.clone(), ExchangePairInfo::new());
        self.conversions
            .insert(pair_string.clone(), conversion.to_string());
        self.pairs
            .insert(pair_string, (pair.0.to_string(), pair.1.to_string()));
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

        let json = Json::from_str(&response_text).unwrap();
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

    // TODO: Implement
    /// Cannot be implemented, because it depends on https://api.icex.ch/api/coins/
    /// which is no working
    /// Temporary solution: hardcode two coins: BTC and ETH
    pub fn get_conversion_coef(&mut self, currency: &str, conversion: &str) -> f64 {
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

                self.worker
                    .lock()
                    .unwrap()
                    .recalculate_total_volume(self.pairs.get(pair).unwrap().0.clone());
            }
        }
    }

    // TODO: Implement
    fn update_market_pair(&mut self, pair: &str, scope: &str, price_changed: bool) {}
}
