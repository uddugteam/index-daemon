#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ConversionType {
    /// Already in USD
    None,

    /// Needed conversion from fiat currency to USD
    _Fiat,

    /// Needed conversion from cryptocurrency to USD
    Crypto,
}

impl Copy for ConversionType {}
