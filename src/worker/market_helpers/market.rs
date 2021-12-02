use crate::worker::market_helpers::market_spine::MarketSpine;
// use crate::worker::markets::binance::Binance;
use crate::worker::markets::bitfinex::Bitfinex;
// use crate::worker::markets::bittrex::Bittrex;
// use crate::worker::markets::poloniex::Poloniex;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

pub fn market_factory(spine: MarketSpine) -> Arc<Mutex<dyn Market + Send>> {
    let market: Arc<Mutex<dyn Market + Send>> = match spine.name.as_ref() {
        // "binance" => Box::new(RefCell::new(Binance { spine, arc: None })),
        "bitfinex" => Arc::new(Mutex::new(Bitfinex { spine, arc: None })),
        // "bittrex" => Box::new(RefCell::new(Bittrex { spine, arc: None })),
        // "poloniex" => Box::new(RefCell::new(Poloniex { spine, arc: None })),
        _ => panic!("Market not found: {}", spine.name),
    };

    market.lock().unwrap().set_arc(Arc::clone(&market));

    market
}

pub trait Market {
    fn set_arc(&mut self, arc: Arc<Mutex<dyn Market + Send>>);
    fn get_spine(&self) -> &MarketSpine;
    fn get_spine_mut(&mut self) -> &mut MarketSpine;
    fn make_pair(&self, pair: (&str, &str)) -> String {
        pair.0.to_string() + pair.1
    }
    fn add_exchange_pair(&mut self, pair: (&str, &str), conversion: &str);
    fn get_total_volume(&self, first_currency: &str, second_currency: &str) -> f64;
    fn update(&mut self) -> Vec<JoinHandle<()>>;
    fn perform(&mut self) -> Vec<JoinHandle<()>> {
        println!("called Market::perform()");

        self.get_spine_mut().refresh_capitalization();
        self.update()
    }
    fn parse_ticker_info__socket(&mut self, pair: String, info: String);
}
