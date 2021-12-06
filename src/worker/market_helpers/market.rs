use crate::worker::market_helpers::market_spine::MarketSpine;
// use crate::worker::markets::binance::Binance;
use crate::worker::markets::bitfinex::Bitfinex;
// use crate::worker::markets::bittrex::Bittrex;
// use crate::worker::markets::poloniex::Poloniex;
use crate::worker::market_helpers::conversion_type::ConversionType;
use std::sync::{Arc, Mutex};

pub fn market_factory(spine: MarketSpine) -> Arc<Mutex<dyn Market + Send>> {
    let market: Arc<Mutex<dyn Market + Send>> = match spine.name.as_ref() {
        // "binance" => Box::new(RefCell::new(Binance { spine, arc: None })),
        "bitfinex" => Arc::new(Mutex::new(Bitfinex { spine })),
        // "bittrex" => Box::new(RefCell::new(Bittrex { spine, arc: None })),
        // "poloniex" => Box::new(RefCell::new(Poloniex { spine, arc: None })),
        _ => panic!("Market not found: {}", spine.name),
    };

    market
        .lock()
        .unwrap()
        .get_spine_mut()
        .set_arc(Arc::clone(&market));

    market
}

pub trait Market {
    fn get_spine(&self) -> &MarketSpine;
    fn get_spine_mut(&mut self) -> &mut MarketSpine;
    fn make_pair(&self, pair: (&str, &str)) -> String {
        pair.0.to_string() + pair.1
    }

    fn add_exchange_pair(&mut self, pair: (&str, &str), conversion: ConversionType) {
        let pair_string = self.make_pair(pair);
        self.get_spine_mut()
            .add_exchange_pair(pair_string, pair, conversion);
    }

    fn get_total_volume(&self, first_currency: &str, second_currency: &str) -> f64 {
        let pair: String = self.make_pair((first_currency, second_currency));
        self.get_spine().get_total_volume(&pair)
    }

    fn get_total_ask(&self, first_currency: &str, second_currency: &str) -> f64 {
        let pair: String = self.make_pair((first_currency, second_currency));

        let ask_sum: f64 = self
            .get_spine()
            .get_exchange_pairs()
            .get(&pair)
            .unwrap()
            .get_total_ask();

        ask_sum
    }

    fn get_total_bid(&self, first_currency: &str, second_currency: &str) -> f64 {
        let pair: String = self.make_pair((first_currency, second_currency));

        let bid_sum: f64 = self
            .get_spine()
            .get_exchange_pairs()
            .get(&pair)
            .unwrap()
            .get_total_bid();

        bid_sum
    }

    fn update(&mut self);
    fn perform(&mut self) {
        println!("called Market::perform()");

        self.get_spine_mut().refresh_capitalization();
        self.update();
    }
    fn parse_ticker_info__socket(&mut self, pair: String, info: String);
    fn parse_last_trade_info__socket(&mut self, pair: String, info: String);
    fn parse_depth_info__socket(&mut self, pair: String, info: String);
}
