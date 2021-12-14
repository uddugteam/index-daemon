use crate::worker::market_helpers::conversion_type::ConversionType;

#[derive(Debug)]
pub struct ExchangePair {
    pub pair: (String, String),
    pub conversion: ConversionType,
}

impl ExchangePair {
    pub fn get_pair_ref(&self) -> (&str, &str) {
        (&self.pair.0, &self.pair.1)
    }
}

impl Clone for ExchangePair {
    fn clone(&self) -> Self {
        Self {
            pair: self.pair.clone(),
            conversion: self.conversion,
        }
    }
}
