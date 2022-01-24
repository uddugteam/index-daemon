use crate::worker::market_helpers::conversion_type::ConversionType;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ExchangePair {
    pub pair: (String, String),
    pub conversion: ConversionType,
}

impl ExchangePair {
    pub fn get_pair_ref(&self) -> (&str, &str) {
        (&self.pair.0, &self.pair.1)
    }
}
