use crate::worker::markets::binance::Binance;
use crate::worker::markets::exchange_pair_info::ExchangePairInfo;
use std::cell::RefCell;
use std::collections::HashMap;
use std::str::FromStr;

pub enum Status {
    Active,
    Down,
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
    status: Status,
    pub name: String,
    api_url: String,
    error_message: String,
    delay: u32,
    mask_pair: HashMap<String, String>,
    unmask_pair: HashMap<String, String>,
    exchange_pair: HashMap<String, ExchangePairInfo>,
    conversions: HashMap<String, String>,
    pairs: HashMap<String, (String, String)>,
    update_ticker: bool,
    update_last_trade: bool,
    update_depth: bool,
    fiat_refresh_time: u64,
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
            mask_pair: HashMap::new(),
            unmask_pair: HashMap::new(),
            exchange_pair: HashMap::new(),
            conversions: HashMap::new(),
            pairs: HashMap::new(),
            update_ticker,
            update_last_trade,
            update_depth,
            fiat_refresh_time,
        }
    }

    pub fn add_mask_pair(&mut self, pair: (&str, &str)) {
        self.mask_pair
            .insert(pair.0.to_string(), pair.1.to_string());
        self.unmask_pair
            .insert(pair.1.to_string(), pair.0.to_string());
    }

    pub fn add_exchange_pair(&mut self, pair_string: String, pair: (&str, &str), conversion: &str) {
        self.exchange_pair
            .insert(pair_string.clone(), ExchangePairInfo::new());
        self.conversions
            .insert(pair_string.clone(), conversion.to_string());
        self.pairs
            .insert(pair_string, (pair.0.to_string(), pair.1.to_string()));
    }

    pub fn get_masked_value<'a>(&'a self, a: &'a str) -> &str {
        self.mask_pair.get(a).map(|s| s.as_ref()).unwrap_or(a)
    }

    pub fn get_unmasked_value<'a>(&'a self, a: &'a str) -> &str {
        self.unmask_pair.get(a).map(|s| s.as_ref()).unwrap_or(a)
    }
}

pub fn market_factory(spine: MarketSpine) -> Option<Box<RefCell<dyn Market>>> {
    match spine.name.as_ref() {
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
}
