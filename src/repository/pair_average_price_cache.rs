use crate::repository::repository::Repository;
use crate::worker::market_helpers::pair_average_price::PairAveragePricePrimaryT;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct PairAveragePriceCache(RefCell<HashMap<PairAveragePricePrimaryT, f64>>);

impl PairAveragePriceCache {
    pub fn _new() -> Self {
        Self(RefCell::new(HashMap::new()))
    }
}

impl Repository<PairAveragePricePrimaryT, f64> for PairAveragePriceCache {
    fn read(&self, primary: PairAveragePricePrimaryT) -> Result<Option<f64>, String> {
        Ok(self.0.borrow_mut().get(&primary).copied())
    }

    fn insert(&self, primary: PairAveragePricePrimaryT, new_value: f64) -> Result<(), String> {
        self.0.borrow_mut().insert(primary, new_value);

        Ok(())
    }

    fn delete(&self, primary: PairAveragePricePrimaryT) {
        self.0.borrow_mut().remove(&primary);
    }
}
