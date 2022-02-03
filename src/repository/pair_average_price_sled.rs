use crate::repository::repository::Repository;
use crate::worker::market_helpers::pair_average_price::PairAveragePricePrimaryT;
use std::sync::{Arc, Mutex};

pub struct PairAveragePriceSled(Arc<Mutex<vsdbsled::Db>>);

impl PairAveragePriceSled {
    pub fn new(db: Arc<Mutex<vsdbsled::Db>>) -> Self {
        Self(db)
    }

    fn make_key(primary: PairAveragePricePrimaryT) -> String {
        let timestamp = primary.0;
        let pair = primary.1;

        format!(
            "{}__worker__pair_average_price__{}_{}",
            timestamp.timestamp_millis(),
            pair.0,
            pair.1,
        )
    }
}

impl Repository<PairAveragePricePrimaryT, f64> for PairAveragePriceSled {
    fn read(&self, primary: PairAveragePricePrimaryT) -> Result<Option<f64>, String> {
        let key = Self::make_key(primary);

        self.0
            .lock()
            .unwrap()
            .get(key)
            .map(|v| v.map(|v| f64::from_ne_bytes(v[0..8].try_into().unwrap())))
            .map_err(|e| e.to_string())
    }

    fn insert(&self, primary: PairAveragePricePrimaryT, new_value: f64) -> Result<(), String> {
        let key = Self::make_key(primary);

        let res = self
            .0
            .lock()
            .unwrap()
            .insert(key, new_value.to_ne_bytes())
            .map(|_| ())
            .map_err(|e| e.to_string());
        let _ = self.0.lock().unwrap().flush();

        res
    }

    fn delete(&self, primary: PairAveragePricePrimaryT) {
        let key = Self::make_key(primary);

        let _ = self.0.lock().unwrap().remove(key);
        let _ = self.0.lock().unwrap().flush();
    }
}
