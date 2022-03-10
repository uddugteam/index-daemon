use crate::config_scheme::market_config::MarketConfig;
use crate::repository::repositories::WorkerRepositoriesByPairTuple;
use crate::worker::market_helpers::market_value::MarketValue;
use crate::worker::market_helpers::stored_and_ws_transmissible_f64::StoredAndWsTransmissibleF64;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channels_holder::WsChannelsHolderHashMap;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type StoredAndWsTransmissibleF64ByPairTuple =
    HashMap<(String, String), Arc<Mutex<StoredAndWsTransmissibleF64>>>;

pub fn make_pair_average_price(
    market_config: &MarketConfig,
    mut repository: Option<WorkerRepositoriesByPairTuple>,
    ws_channels_holder: &WsChannelsHolderHashMap,
) -> StoredAndWsTransmissibleF64ByPairTuple {
    let mut hash_map = HashMap::new();

    for exchange_pair in &market_config.exchange_pairs {
        let pair = exchange_pair.pair.clone();
        let key = (
            "worker".to_string(),
            MarketValue::PairAveragePrice,
            Some(pair.clone()),
        );
        let ws_channels = ws_channels_holder.get(&key).unwrap();

        let pair_average_price = Arc::new(Mutex::new(StoredAndWsTransmissibleF64::new(
            repository
                .as_mut()
                .map(|v| v.remove(&exchange_pair.pair).unwrap()),
            vec![
                WsChannelName::CoinAveragePrice,
                WsChannelName::CoinAveragePriceCandles,
            ],
            None,
            Some(pair.clone()),
            Arc::clone(ws_channels),
        )));

        hash_map.insert(pair, pair_average_price);
    }

    hash_map
}
