use crate::worker::market_helpers::exchange_pair_info::ExchangePairInfo;
use crate::worker::market_helpers::status::Status;
use std::collections::HashMap;

pub struct MarketSpine {
    pub status: Status,
    pub name: String,
    api_url: String,
    error_message: String,
    pub delay: u32,
    mask_pairs: HashMap<String, String>,
    unmask_pairs: HashMap<String, String>,
    exchange_pairs: HashMap<String, ExchangePairInfo>,
    conversions: HashMap<String, String>,
    pairs: HashMap<String, (String, String)>,
    update_ticker: bool,
    update_last_trade: bool,
    update_depth: bool,
    fiat_refresh_time: u64,
    pub socket_enabled: bool,
}
impl MarketSpine {
    pub fn new(
        status: Status,
        name: String,
        api_url: String,
        error_message: String,
        delay: u32,
        update_ticker: bool,
        update_last_trade: bool,
        update_depth: bool,
        fiat_refresh_time: u64,
    ) -> Self {
        Self {
            status,
            name,
            api_url,
            error_message,
            delay,
            mask_pairs: HashMap::new(),
            unmask_pairs: HashMap::new(),
            exchange_pairs: HashMap::new(),
            conversions: HashMap::new(),
            pairs: HashMap::new(),
            update_ticker,
            update_last_trade,
            update_depth,
            fiat_refresh_time,
            socket_enabled: false,
        }
    }

    pub fn add_mask_pair(&mut self, pair: (&str, &str)) {
        self.mask_pairs
            .insert(pair.0.to_string(), pair.1.to_string());
        self.unmask_pairs
            .insert(pair.1.to_string(), pair.0.to_string());
    }

    pub fn get_exchange_pairs(&self) -> &HashMap<String, ExchangePairInfo> {
        &self.exchange_pairs
    }

    pub fn get_exchange_pairs_mut(&mut self) -> &mut HashMap<String, ExchangePairInfo> {
        &mut self.exchange_pairs
    }

    pub fn add_exchange_pair(&mut self, pair_string: String, pair: (&str, &str), conversion: &str) {
        self.exchange_pairs
            .insert(pair_string.clone(), ExchangePairInfo::new());
        self.conversions
            .insert(pair_string.clone(), conversion.to_string());
        self.pairs
            .insert(pair_string, (pair.0.to_string(), pair.1.to_string()));
    }

    pub fn get_masked_value<'a>(&'a self, a: &'a str) -> &str {
        self.mask_pairs.get(a).map(|s| s.as_ref()).unwrap_or(a)
    }

    pub fn get_unmasked_value<'a>(&'a self, a: &'a str) -> &str {
        self.unmask_pairs.get(a).map(|s| s.as_ref()).unwrap_or(a)
    }

    // TODO: Implement
    pub fn refresh_capitalization(&self) {}
}
