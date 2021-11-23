use crate::worker::market_helpers::exchange_pair_info::ExchangePairInfo;
use crate::worker::markets::binance::Binance;
// use crate::worker::markets::bittrex::Bittrex;
// use crate::worker::markets::poloniex::Poloniex;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

pub enum Status {
    Active,
    Down,
}
impl Status {
    pub fn is_active(&self) -> bool {
        matches!(self, Status::Active)
    }

    pub fn is_down(&self) -> bool {
        matches!(self, Status::Down)
    }
}
// TODO: Replace error type with correct
impl FromStr for Status {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "down" => Ok(Self::Down),
            _ => Err(()),
        }
    }
}
impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let status_string = match self {
            Status::Active => "active",
            Status::Down => "down",
        };

        write!(f, "{}", status_string)
    }
}
impl Clone for Status {
    fn clone(&self) -> Self {
        match self {
            Self::Active => Self::Active,
            Self::Down => Self::Down,
        }
    }
}
impl Copy for Status {}

pub struct MarketSpine {
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
}
impl MarketSpine {
    pub fn new(
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
        }
    }

    pub fn add_mask_pair(&mut self, pair: (&str, &str)) {
        self.mask_pairs
            .insert(pair.0.to_string(), pair.1.to_string());
        self.unmask_pairs
            .insert(pair.1.to_string(), pair.0.to_string());
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

    // TODO: Implement
    pub fn refresh_capitalization(&self) {}
}

pub fn market_factory(spine: MarketSpine) -> Option<Box<RefCell<dyn Market + Send>>> {
    match spine.name.as_ref() {
        // "poloniex" => Some(Box::new(RefCell::new(Poloniex { spine }))),
        // "bittrex" => Some(Box::new(RefCell::new(Bittrex { spine }))),
        "binance" => Some(Box::new(RefCell::new(Binance { spine }))),
        _ => None,
    }
}

pub trait Market {
    fn get_spine(&self) -> &MarketSpine;
    fn get_spine_mut(&mut self) -> &mut MarketSpine;
    fn make_pair(&self, pair: (&str, &str)) -> String {
        pair.0.to_string() + pair.1
    }
    fn add_exchange_pair(&mut self, pair: (&str, &str), conversion: &str);
    fn update(&mut self);
    fn perform(&mut self) {
        println!("called Market::perform()");

        self.get_spine().refresh_capitalization();
        self.update();
    }
}
