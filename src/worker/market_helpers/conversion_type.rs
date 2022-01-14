#[derive(Debug, PartialEq, Eq)]
pub enum ConversionType {
    /// Already in USD
    None,

    /// Needed conversion from fiat currency to USD
    _Fiat,

    /// Needed conversion from cryptocurrency to USD
    Crypto,
}

impl Clone for ConversionType {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for ConversionType {}
