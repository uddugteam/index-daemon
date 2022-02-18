use crate::config_scheme::market_config::MarketConfig;
use crate::config_scheme::service_config::ServiceConfig;
use clap::{App, Arg, ArgMatches, ValueHint};

#[derive(Default)]
pub struct ConfigScheme {
    pub market: MarketConfig,
    pub service: ServiceConfig,
    pub matches: ArgMatches,
}

impl ConfigScheme {
    pub fn new() -> Self {
        let matches = Self::make_matches();

        Self {
            market: MarketConfig::new(&matches),
            service: ServiceConfig::new(&matches),
            matches,
        }
    }

    /// Call only once
    fn make_matches() -> ArgMatches {
        App::new("ICEX")
            .version("1.0")
            .arg(
                Arg::new("service_config")
                    .long("service_config")
                    .value_name("PATH")
                    .help("Service config file path")
                    .value_hint(ValueHint::FilePath),
            )
            .arg(
                Arg::new("market_config")
                    .long("market_config")
                    .value_name("PATH")
                    .help("Market config file path")
                    .value_hint(ValueHint::FilePath),
            )
            .get_matches()
    }
}
