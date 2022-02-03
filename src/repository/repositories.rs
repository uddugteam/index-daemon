use crate::repository::f64_by_timestamp_and_pair_tuple_sled::{
    F64ByTimestampAndPairTupleSled, TimestampAndPairTuple,
};
use crate::repository::repository::Repository;
use std::sync::{Arc, Mutex};

pub struct Repositories {
    pub pair_average_price: Box<dyn Repository<TimestampAndPairTuple, f64> + Send>,
}

impl Repositories {
    pub fn new() -> Self {
        let tree = Arc::new(Mutex::new(vsdbsled::open("db").expect("Open db error.")));
        let pair_average_price = Box::new(F64ByTimestampAndPairTupleSled::new(
            "worker__pair_average_price".to_string(),
            Arc::clone(&tree),
        ));

        Self { pair_average_price }
    }
}
