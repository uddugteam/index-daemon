use crate::config_scheme::config_scheme::ConfigScheme;
use crate::repository::repositories::WorkerRepositoriesByPairTuple;
use crate::worker::market_helpers::market_value::MarketValue;
use crate::worker::market_helpers::market_value_owner::MarketValueOwner;
use crate::worker::market_helpers::percent_change::PercentChangeByInterval;
use crate::worker::market_helpers::stored_and_ws_transmissible_f64::StoredAndWsTransmissibleF64;
use crate::worker::network_helpers::ws_server::holders::helper_functions::HolderHashMap;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type StoredAndWsTransmissibleF64ByPairTuple =
    HashMap<(String, String), Arc<RwLock<StoredAndWsTransmissibleF64>>>;

pub fn make_pair_average_price(
    config: &ConfigScheme,
    mut repository: Option<WorkerRepositoriesByPairTuple>,
    percent_change_holder: &HolderHashMap<PercentChangeByInterval>,
    ws_channels_holder: &HolderHashMap<WsChannels>,
) -> StoredAndWsTransmissibleF64ByPairTuple {
    let mut hash_map = HashMap::new();

    for exchange_pair in &config.market.exchange_pairs {
        let pair = exchange_pair.clone();
        let key = (
            MarketValueOwner::Worker,
            MarketValue::PairAveragePrice,
            Some(pair.clone()),
        );
        let percent_change = percent_change_holder.get(&key).unwrap();
        let ws_channels = ws_channels_holder.get(&key).unwrap();

        let pair_average_price = Arc::new(RwLock::new(StoredAndWsTransmissibleF64::new(
            repository
                .as_mut()
                .map(|v| v.remove(exchange_pair).unwrap()),
            vec![
                WsChannelName::CoinAveragePrice,
                WsChannelName::CoinAveragePriceCandles,
            ],
            None,
            Some(pair.clone()),
            Arc::clone(percent_change),
            config.service.percent_change_interval_sec,
            Arc::clone(ws_channels),
        )));

        hash_map.insert(pair, pair_average_price);
    }

    hash_map
}
