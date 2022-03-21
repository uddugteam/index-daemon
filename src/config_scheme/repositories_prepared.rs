use crate::config_scheme::config_scheme::ConfigScheme;
use crate::repository::repositories::{
    MarketRepositoriesByMarketName, Repositories, RepositoryForF64ByTimestamp,
    WorkerRepositoriesByPairTuple,
};
use crate::worker::market_helpers::market_value::MarketValue;
use crate::worker::market_helpers::pair_average_price::{
    make_pair_average_price, StoredAndWsTransmissibleF64ByPairTuple,
};
use crate::worker::market_helpers::stored_and_ws_transmissible_f64::StoredAndWsTransmissibleF64;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channels_holder::{
    WsChannelsHolder, WsChannelsHolderHashMap,
};
use std::sync::{Arc, Mutex};

pub struct RepositoriesPrepared {
    pub index_price_repository: Option<RepositoryForF64ByTimestamp>,
    pub pair_average_price_repositories: Option<WorkerRepositoriesByPairTuple>,
    pub market_repositories: Option<MarketRepositoriesByMarketName>,
    pub ws_channels_holder: WsChannelsHolderHashMap,
    pub pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
    pub index_price: Arc<Mutex<StoredAndWsTransmissibleF64>>,
}

impl RepositoriesPrepared {
    pub fn make(config: &ConfigScheme) -> Self {
        let (pair_average_price_repositories, market_repositories, index_price_repository) =
            Repositories::optionize_fields(Repositories::new(config));

        let ws_channels_holder = WsChannelsHolder::make_hashmap(&config.market);

        let pair_average_price = make_pair_average_price(
            &config.market,
            pair_average_price_repositories.clone(),
            &ws_channels_holder,
        );

        let index_price =
            Self::make_index_price(index_price_repository.clone(), &ws_channels_holder);

        Self {
            index_price_repository,
            pair_average_price_repositories,
            market_repositories,
            ws_channels_holder,
            pair_average_price,
            index_price,
        }
    }

    fn make_index_price(
        index_repository: Option<RepositoryForF64ByTimestamp>,
        ws_channels_holder: &WsChannelsHolderHashMap,
    ) -> Arc<Mutex<StoredAndWsTransmissibleF64>> {
        let key = ("worker".to_string(), MarketValue::IndexPrice, None);
        let ws_channels = ws_channels_holder.get(&key).unwrap();

        Arc::new(Mutex::new(StoredAndWsTransmissibleF64::new(
            index_repository,
            vec![WsChannelName::IndexPrice, WsChannelName::IndexPriceCandles],
            None,
            None,
            Arc::clone(ws_channels),
        )))
    }
}
