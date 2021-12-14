use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug)]
pub enum ConversionType {
    None,
    Fiat,
    Crypto,
}

// TODO: Replace error type with correct
impl FromStr for ConversionType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(Self::None),
            "fiat" => Ok(Self::Fiat),
            "crypto" => Ok(Self::Crypto),
            _ => Err(()),
        }
    }
}
impl Display for ConversionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let status_string = match self {
            ConversionType::None => "none",
            ConversionType::Fiat => "fiat",
            ConversionType::Crypto => "crypto",
        };

        write!(f, "{}", status_string)
    }
}
impl Clone for ConversionType {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for ConversionType {}
