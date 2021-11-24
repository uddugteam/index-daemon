use crate::worker::market_helpers::action::Action;
use crate::worker::markets::binance::Binance;
// use crate::worker::markets::bittrex::Bittrex;
// use crate::worker::markets::poloniex::Poloniex;
use crate::worker::market_helpers::market_name::MarketName;
use crate::worker::market_helpers::market_spine::MarketSpine;
use std::cell::RefCell;

pub fn market_factory(spine: MarketSpine) -> Option<Box<RefCell<dyn Market + Send>>> {
    match spine.name.as_ref() {
        // "poloniex" => Some(Box::new(RefCell::new(Poloniex { spine }))),
        // "bittrex" => Some(Box::new(RefCell::new(Bittrex { spine }))),
        "binance" => Some(Box::new(RefCell::new(Binance { spine }))),
        _ => None,
    }
}

pub trait Market {
    fn get_name(&self) -> MarketName;
    fn get_spine(&self) -> &MarketSpine;
    fn get_spine_mut(&mut self) -> &mut MarketSpine;
    fn make_pair(&self, pair: (&str, &str)) -> String {
        pair.0.to_string() + pair.1
    }
    fn add_exchange_pair(&mut self, pair: (&str, &str), conversion: &str);
    fn update(&mut self) -> Vec<Action>;
    fn perform(&mut self) -> Vec<Action> {
        println!("called Market::perform()");

        self.get_spine().refresh_capitalization();
        self.update()
    }
}
