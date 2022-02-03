use crate::repository::pair_average_price_sled::PairAveragePriceSled;
use crate::repository::repository::Repository;
use crate::worker::market_helpers::pair_average_price::PairAveragePricePrimaryT;
use std::sync::{Arc, Mutex};

pub struct Repositories {
    pub pair_average_price: Box<dyn Repository<PairAveragePricePrimaryT, f64> + Send>,
}

impl Repositories {
    pub fn new() -> Self {
        let tree = Arc::new(Mutex::new(vsdbsled::open("db").expect("Open db error.")));
        let pair_average_price = Box::new(PairAveragePriceSled::new(Arc::clone(&tree)));

        Self { pair_average_price }
    }
}
