use crate::worker::markets::market::{Market, MarketSpine};

pub struct Binance {
    pub spine: MarketSpine,
}

impl Market for Binance {
    fn get_spine(&self) -> &MarketSpine {
        &self.spine
    }

    fn get_spine_mut(&mut self) -> &mut MarketSpine {
        &mut self.spine
    }

    fn make_pair(&self, pair: (&str, &str)) -> String {
        (self.spine.get_masked_value(pair.0).to_string() + self.spine.get_masked_value(pair.1))
            .to_lowercase()
    }

    fn add_exchange_pair(&mut self, pair: (&str, &str), conversion: &str) {
        let pair_string = self.make_pair(pair);
        self.spine.add_exchange_pair(pair_string, pair, conversion);
    }
}
