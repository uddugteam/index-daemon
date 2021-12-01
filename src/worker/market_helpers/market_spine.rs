use crate::worker::market_helpers::exchange_pair_info::ExchangePairInfo;
use crate::worker::market_helpers::status::Status;
use std::collections::HashMap;

const EPS: f64 = 0.00001;

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
    recalculate_total_volume_handler: Box<dyn FnOnce(String) + Send>,
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
        recalculate_total_volume_handler: Box<dyn FnOnce(String) + Send>,
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
            recalculate_total_volume_handler,
        }
    }

    pub fn add_mask_pair(&mut self, pair: (&str, &str)) {
        self.mask_pairs
            .insert(pair.0.to_string(), pair.1.to_string());
        self.unmask_pairs
            .insert(pair.1.to_string(), pair.0.to_string());
    }

    pub fn get_pairs(&self) -> &HashMap<String, (String, String)> {
        &self.pairs
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

    // TODO: Implement
    pub fn get_conversion_coef(&mut self, currency: &str, conversion: &str) -> f64 {
        1.0
    }

    // TODO: Implement
    pub fn set_total_volume(&mut self, pair: &str, value: f64) {
        if value != 0.0 {
            let old_value: f64 = self.exchange_pairs.get(pair).unwrap().get_total_volume();

            if (old_value - value).abs() > EPS {
                self.exchange_pairs
                    .get_mut(pair)
                    .unwrap()
                    .set_total_volume(value.abs());
                self.update_market_pair(pair, "totalValues", false);

                // C++: this->reCalculateTotalVolumeHandler(pairs.at(pair).first);
            }
        }
    }

    // TODO: Implement
    fn update_market_pair(&mut self, pair: &str, scope: &str, price_changed: bool) {}
}
