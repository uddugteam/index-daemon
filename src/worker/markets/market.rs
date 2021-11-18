use crate::worker::markets::binance::Binance;
use std::str::FromStr;

pub enum Status {
    Active,
    Down,
}
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
    pub status: Status,
    pub name: String,
    pub api_url: String,
    pub error_message: String,
    pub delay: u32,
}
impl MarketSpine {}

pub fn marketFactory(spine: MarketSpine) -> Option<Box<dyn Market>> {
    match spine.name.as_ref() {
        "binance" => Some(Box::new(Binance { spine })),
        _ => None,
    }
}

pub trait Market {
    fn get_spine(&self) -> &MarketSpine;
}
