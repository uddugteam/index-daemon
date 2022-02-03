#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ConversionType {
    /// Already in USD
    None,

    /// Needed conversion from fiat currency to USD
    _Fiat,

    /// Needed conversion from cryptocurrency to USD
    Crypto,
}
