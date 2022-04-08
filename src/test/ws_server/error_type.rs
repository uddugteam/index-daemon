use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;

#[derive(Debug, Copy, Clone)]
pub enum Field {
    Id,
    Method,
    Params,
    Coins,
    Exchanges,
    FrequencyMs,
    PercentChangeInterval,
    Interval,
}

impl Field {
    pub fn is_root(&self) -> bool {
        matches!(self, Self::Id | Self::Method | Self::Params)
    }

    fn get_by_channel(channel: WsChannelName) -> Vec<Field> {
        match channel {
            WsChannelName::IndexPrice => vec![Self::FrequencyMs, Self::PercentChangeInterval],
            WsChannelName::IndexPriceCandles => vec![Self::FrequencyMs, Self::Interval],
            WsChannelName::CoinAveragePrice => {
                vec![Self::Coins, Self::FrequencyMs, Self::PercentChangeInterval]
            }
            WsChannelName::CoinAveragePriceCandles => {
                vec![Self::Coins, Self::FrequencyMs, Self::Interval]
            }
            WsChannelName::CoinExchangePrice => {
                vec![
                    Self::Coins,
                    Self::Exchanges,
                    Self::FrequencyMs,
                    Self::PercentChangeInterval,
                ]
            }
            WsChannelName::CoinExchangeVolume => {
                vec![
                    Self::Coins,
                    Self::Exchanges,
                    Self::FrequencyMs,
                    Self::PercentChangeInterval,
                ]
            }
            _ => unreachable!(),
        }
    }
}

impl ToString for Field {
    fn to_string(&self) -> String {
        match self {
            Self::Id => "id",
            Self::Method => "method",
            Self::Params => "params",
            Self::Coins => "coins",
            Self::Exchanges => "exchanges",
            Self::FrequencyMs => "frequency_ms",
            Self::PercentChangeInterval => "percent_change_interval",
            Self::Interval => "interval",
        }
        .to_string()
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ErrorType {
    Lack(Field),
    Null(Field),
    Empty(Field),
    InvalidType(Field),
    InvalidValue(Field),
    Low(Field),
    Unavailable(Field),
}

impl ErrorType {
    fn get_general() -> Vec<Self> {
        vec![
            // -----------------------------------------------------------
            Self::Lack(Field::Id),
            Self::Null(Field::Id),
            Self::InvalidType(Field::Id),
            // -----------------------------------------------------------
            Self::Lack(Field::Method),
            Self::Null(Field::Method),
            Self::InvalidType(Field::Method),
            Self::Unavailable(Field::Method),
            // -----------------------------------------------------------
            Self::Lack(Field::Params),
            Self::Null(Field::Params),
            Self::Empty(Field::Params),
            Self::InvalidType(Field::Params),
            // -----------------------------------------------------------
        ]
    }

    fn get_by_field(field: Field) -> Vec<Self> {
        match field {
            Field::Coins => {
                vec![
                    Self::Lack(field),
                    Self::Null(field),
                    Self::Empty(field),
                    Self::InvalidType(field),
                    Self::Unavailable(field),
                ]
            }
            Field::Exchanges => {
                vec![
                    Self::Lack(field),
                    Self::Null(field),
                    Self::Empty(field),
                    Self::InvalidType(field),
                    Self::Unavailable(field),
                ]
            }
            Field::FrequencyMs => {
                vec![Self::InvalidType(field), Self::Low(field)]
            }
            Field::PercentChangeInterval => {
                vec![
                    Self::InvalidType(field),
                    Self::InvalidValue(field),
                    Self::Low(field),
                ]
            }
            Field::Interval => {
                vec![
                    Self::Lack(field),
                    Self::Null(field),
                    Self::InvalidType(field),
                    Self::InvalidValue(field),
                    Self::Low(field),
                ]
            }
            _ => unreachable!(),
        }
    }

    pub fn get_by_channel(channel: WsChannelName) -> Vec<Self> {
        let mut res = Vec::new();

        res.append(&mut Self::get_general());

        for field in Field::get_by_channel(channel) {
            res.append(&mut Self::get_by_field(field));
        }

        res
    }
}
