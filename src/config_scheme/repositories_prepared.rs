use crate::config_scheme::config_scheme::ConfigScheme;
use crate::repository::repositories::{
    MarketRepositoriesByMarketName, Repositories, RepositoryForF64ByTimestamp,
    WorkerRepositoriesByPairTuple,
};
use crate::worker::market_helpers::market_value::MarketValue;
use crate::worker::market_helpers::market_value_owner::MarketValueOwner;
use crate::worker::market_helpers::pair_average_price::{
    make_pair_average_price, StoredAndWsTransmissibleF64ByPairTuple,
};
use crate::worker::market_helpers::percent_change_by_interval::PercentChangeByInterval;
use crate::worker::market_helpers::stored_and_ws_transmissible_f64::StoredAndWsTransmissibleF64;
use crate::worker::network_helpers::ws_server::holders::helper_functions::make_holder_hashmap;
use crate::worker::network_helpers::ws_server::holders::helper_functions::HolderHashMap;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct RepositoriesPrepared {
    pub index_price_repository: Option<RepositoryForF64ByTimestamp>,
    pub pair_average_price_repositories: Option<WorkerRepositoriesByPairTuple>,
    pub market_repositories: Option<MarketRepositoriesByMarketName>,
    pub percent_change_holder: HolderHashMap<PercentChangeByInterval>,
    pub ws_channels_holder: HolderHashMap<WsChannels>,
    pub index_price: Arc<RwLock<StoredAndWsTransmissibleF64>>,
    pub pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
}

impl RepositoriesPrepared {
    pub fn make(config: &ConfigScheme) -> Self {
        let (pair_average_price_repositories, market_repositories, index_price_repository) =
            Repositories::optionize_fields(Repositories::new(config));

        let percent_change_holder = make_holder_hashmap::<PercentChangeByInterval>(config);
        let ws_channels_holder = make_holder_hashmap::<WsChannels>(config);

        let pair_average_price = make_pair_average_price(
            config,
            pair_average_price_repositories.clone(),
            &percent_change_holder,
            &ws_channels_holder,
        );

        let index_price = Self::make_index_price(
            index_price_repository.clone(),
            &percent_change_holder,
            config.service.percent_change_interval_sec,
            &ws_channels_holder,
        );

        Self {
            index_price_repository,
            pair_average_price_repositories,
            market_repositories,
            percent_change_holder,
            ws_channels_holder,
            index_price,
            pair_average_price,
        }
    }

    fn make_index_price(
        index_repository: Option<RepositoryForF64ByTimestamp>,
        percent_change_holder: &HolderHashMap<PercentChangeByInterval>,
        percent_change_interval_sec: u64,
        ws_channels_holder: &HolderHashMap<WsChannels>,
    ) -> Arc<RwLock<StoredAndWsTransmissibleF64>> {
        let key = (MarketValueOwner::Worker, MarketValue::IndexPrice, None);
        let percent_change = percent_change_holder.get(&key).unwrap();
        let ws_channels = ws_channels_holder.get(&key).unwrap();

        Arc::new(RwLock::new(StoredAndWsTransmissibleF64::new(
            index_repository,
            vec![WsChannelName::IndexPrice, WsChannelName::IndexPriceCandles],
            None,
            None,
            Arc::clone(percent_change),
            percent_change_interval_sec,
            Arc::clone(ws_channels),
        )))
    }
}
