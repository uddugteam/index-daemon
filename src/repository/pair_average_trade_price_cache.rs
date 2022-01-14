use crate::repository::repository::Repository;
use std::collections::HashMap;

pub struct PairAverageTradePriceCache(HashMap<(String, String), f64>);

impl PairAverageTradePriceCache {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

impl Repository<(String, String), f64> for PairAverageTradePriceCache {
    fn read(&self, primary: (String, String)) -> Option<f64> {
        self.0.get(&primary).copied()
    }

    fn insert(&mut self, primary: (String, String), new_value: f64) {
        self.0.insert(primary, new_value);
    }

    fn delete(&mut self, primary: (String, String)) {
        self.0.remove(&primary);
    }
}
