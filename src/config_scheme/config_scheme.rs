use crate::config_scheme::market_config::MarketConfig;
use crate::config_scheme::service_config::ServiceConfig;

pub struct ConfigScheme {
    pub market: MarketConfig,
    pub service: ServiceConfig,
}
impl ConfigScheme {
    pub fn new() -> Self {
        Self {
            market: MarketConfig::new(),
            service: ServiceConfig::new(),
        }
    }
}
impl Default for ConfigScheme {
    fn default() -> Self {
        Self {
            market: MarketConfig::default(),
            service: ServiceConfig::default(),
        }
    }
}
