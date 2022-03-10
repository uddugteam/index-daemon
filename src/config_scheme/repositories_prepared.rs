use crate::config_scheme::config_scheme::ConfigScheme;
use crate::repository::repositories::{
    MarketRepositoriesByMarketName, Repositories, WorkerRepositoriesByPairTuple,
};
use crate::worker::market_helpers::pair_average_price::{
    make_pair_average_price, StoredAndWsTransmissibleF64ByPairTuple,
};
use crate::worker::network_helpers::ws_server::ws_channels_holder::{
    WsChannelsHolder, WsChannelsHolderHashMap,
};

pub struct RepositoriesPrepared {
    pub pair_average_price_repository: Option<WorkerRepositoriesByPairTuple>,
    pub market_repositories: Option<MarketRepositoriesByMarketName>,
    pub ws_channels_holder: WsChannelsHolderHashMap,
    pub pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
}

impl RepositoriesPrepared {
    pub fn make(config: &ConfigScheme) -> Self {
        let (pair_average_price_repository, market_repositories) =
            Repositories::optionize_fields(Repositories::new(config));

        let ws_channels_holder = WsChannelsHolder::make_hashmap(&config.market);

        let pair_average_price = make_pair_average_price(
            &config.market,
            pair_average_price_repository.clone(),
            &ws_channels_holder,
        );

        Self {
            pair_average_price_repository,
            market_repositories,
            ws_channels_holder,
            pair_average_price,
        }
    }
}
