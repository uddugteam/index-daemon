use crate::worker::market_helpers::conversion_type::ConversionType;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ExchangePair {
    pub pair: (String, String),
    pub conversion: ConversionType,
}
