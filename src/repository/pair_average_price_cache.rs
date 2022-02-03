use crate::repository::repository::Repository;
use crate::worker::market_helpers::pair_average_price::TimestampAndPairTuple;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct PairAveragePriceCache(RefCell<HashMap<TimestampAndPairTuple, f64>>);

impl PairAveragePriceCache {
    pub fn _new() -> Self {
        Self(RefCell::new(HashMap::new()))
    }
}

impl Repository<TimestampAndPairTuple, f64> for PairAveragePriceCache {
    fn read(&self, primary: TimestampAndPairTuple) -> Result<Option<f64>, String> {
        Ok(self.0.borrow_mut().get(&primary).copied())
    }

    fn insert(&self, primary: TimestampAndPairTuple, new_value: f64) -> Result<(), String> {
        self.0.borrow_mut().insert(primary, new_value);

        Ok(())
    }

    fn delete(&self, primary: TimestampAndPairTuple) {
        self.0.borrow_mut().remove(&primary);
    }
}
