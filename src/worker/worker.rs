use crate::config_scheme::config_scheme::ConfigScheme;
use crate::config_scheme::market_config::MarketConfig;
use crate::config_scheme::repositories_prepared::RepositoriesPrepared;
use crate::config_scheme::service_config::ServiceConfig;
use crate::graceful_shutdown::GracefulShutdown;
use crate::helper_functions::fill_historical_data;
use crate::repository::repositories::{
    MarketRepositoriesByMarketName, RepositoryForF64ByTimestamp, WorkerRepositoriesByPairTuple,
};
use crate::worker::db_cleaner::clear_db;
use crate::worker::market_helpers::market::{market_factory, market_update, Market};
use crate::worker::market_helpers::market_channels::ExternalMarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;
use crate::worker::market_helpers::pair_average_price::StoredAndWsTransmissibleF64ByPairTuple;
use crate::worker::market_helpers::percent_change_by_interval::PercentChangeByInterval;
use crate::worker::market_helpers::stored_and_ws_transmissible_f64::StoredAndWsTransmissibleF64;
use crate::worker::network_helpers::ws_server::holders::helper_functions::HolderHashMap;
use crate::worker::network_helpers::ws_server::holders::percent_change_holder::PercentChangeByIntervalHolder;
use crate::worker::network_helpers::ws_server::holders::ws_channels_holder::WsChannelsHolder;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use crate::worker::network_helpers::ws_server::ws_server::WsServer;
use futures::FutureExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

async fn configure(
    market_names: Vec<&str>,
    exchange_pairs: Vec<(String, String)>,
    channels: Vec<ExternalMarketChannels>,
    rest_timeout_sec: u64,
    repositories: Option<MarketRepositoriesByMarketName>,
    pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
    index_price: Arc<RwLock<StoredAndWsTransmissibleF64>>,
    index_pairs: Vec<(String, String)>,
    percent_change_holder: &HolderHashMap<PercentChangeByInterval>,
    percent_change_interval_sec: u64,
    ws_channels_holder: &HolderHashMap<WsChannels>,
    graceful_shutdown: GracefulShutdown,
) -> HashMap<String, Arc<Mutex<dyn Market + Send + Sync>>> {
    let mut markets = HashMap::new();
    let mut repositories = repositories.unwrap_or_default();

    for market_name in market_names {
        let pair_average_price_2 = pair_average_price.clone();
        let market_spine = MarketSpine::new(
            pair_average_price_2,
            Arc::clone(&index_price),
            index_pairs.clone(),
            rest_timeout_sec,
            market_name.to_string(),
            channels.clone(),
            graceful_shutdown.clone(),
        );
        let market = market_factory(
            market_spine,
            exchange_pairs.clone(),
            repositories.remove(market_name),
            percent_change_holder,
            percent_change_interval_sec,
            ws_channels_holder,
        )
        .await
        .unwrap();

        markets.insert(market_name.to_string(), market);
    }

    markets
}

async fn start_ws(
    ws_addr: String,
    ws_answer_timeout_ms: u64,
    percent_change_interval_sec: u64,
    index_price_repository: Option<RepositoryForF64ByTimestamp>,
    pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
    market_repositories: Option<MarketRepositoriesByMarketName>,
    percent_change_holder: HolderHashMap<PercentChangeByInterval>,
    ws_channels_holder: HolderHashMap<WsChannels>,
    graceful_shutdown: GracefulShutdown,
) {
    let percent_change_holder = PercentChangeByIntervalHolder::new(percent_change_holder);
    let ws_channels_holder = WsChannelsHolder::new(ws_channels_holder);

    let ws_server = WsServer {
        percent_change_holder,
        ws_channels_holder,
        ws_addr,
        ws_answer_timeout_ms,
        percent_change_interval_sec,
        index_price_repository,
        pair_average_price,
        market_repositories,
        graceful_shutdown,
    };

    ws_server.run().await;
}

async fn start_db_cleaner(
    index_price_repository: Option<RepositoryForF64ByTimestamp>,
    pair_average_price_repositories: Option<WorkerRepositoriesByPairTuple>,
    market_repositories: Option<MarketRepositoriesByMarketName>,
    data_expire_sec: u64,
) {
    loop {
        clear_db(
            index_price_repository.clone(),
            pair_average_price_repositories.clone(),
            market_repositories.clone(),
            data_expire_sec,
        )
        .await;

        // Sleep 1 day
        sleep(Duration::from_secs(86_400)).await;
    }
}

pub async fn start_worker(
    config: ConfigScheme,
    repositories_prepared: RepositoriesPrepared,
    graceful_shutdown: GracefulShutdown,
) {
    let mut futures = Vec::new();

    let RepositoriesPrepared {
        index_price_repository,
        pair_average_price_repositories,
        market_repositories,
        percent_change_holder,
        ws_channels_holder,
        index_price,
        pair_average_price,
    } = repositories_prepared;

    fill_historical_data(
        &config,
        index_price_repository.clone(),
        pair_average_price_repositories.clone(),
    )
    .await;

    let ConfigScheme {
        market,
        service,
        matches: _,
    } = config;
    let MarketConfig {
        markets,
        exchange_pairs,
        index_pairs,
        channels,
    } = market;
    let ServiceConfig {
        rest_timeout_sec,
        ws,
        ws_addr,
        ws_answer_timeout_ms,
        storage: _,
        historical_storage_frequency_ms: _,
        data_expire_sec,
        percent_change_interval_sec,
    } = service;

    let markets = markets.iter().map(|v| v.as_ref()).collect();

    let markets = configure(
        markets,
        exchange_pairs,
        channels,
        rest_timeout_sec,
        market_repositories.clone(),
        pair_average_price.clone(),
        index_price,
        index_pairs,
        &percent_change_holder,
        percent_change_interval_sec,
        &ws_channels_holder,
        graceful_shutdown.clone(),
    )
    .await;

    if ws {
        let future = start_ws(
            ws_addr,
            ws_answer_timeout_ms,
            percent_change_interval_sec,
            index_price_repository.clone(),
            pair_average_price,
            market_repositories.clone(),
            percent_change_holder,
            ws_channels_holder,
            graceful_shutdown.clone(),
        );

        let future = tokio::spawn(future).map(|v| v.unwrap());
        futures.push(future.boxed());
    }

    let future = start_db_cleaner(
        index_price_repository,
        pair_average_price_repositories,
        market_repositories,
        data_expire_sec,
    );
    futures.push(future.boxed());

    for (_market_name, market) in markets {
        if graceful_shutdown.get().await {
            return;
        }

        let future = market_update(market);
        let future = tokio::spawn(future).map(|v| v.unwrap());
        futures.push(future.boxed());
    }

    futures::future::join_all(futures).await;
}

#[cfg(test)]
pub mod test {
    use crate::config_scheme::config_scheme::ConfigScheme;
    use crate::config_scheme::helper_functions::make_exchange_pairs;
    use crate::config_scheme::market_config::MarketConfig;
    use crate::config_scheme::repositories_prepared::RepositoriesPrepared;
    use crate::graceful_shutdown::GracefulShutdown;
    use crate::worker::market_helpers::market_channels::ExternalMarketChannels;
    use crate::worker::worker::configure;
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::thread::JoinHandle;

    pub fn prepare_worker_params() -> (
        Sender<JoinHandle<()>>,
        Receiver<JoinHandle<()>>,
        GracefulShutdown,
    ) {
        let (tx, rx) = mpsc::channel();
        let graceful_shutdown = GracefulShutdown::new();

        (tx, rx, graceful_shutdown)
    }

    async fn test_configure(
        market_names: Vec<&str>,
        exchange_pairs: Vec<(String, String)>,
        channels: Vec<ExternalMarketChannels>,
    ) {
        let (_, _, graceful_shutdown) = prepare_worker_params();

        let mut config = ConfigScheme::default();
        config.market.markets = market_names.iter().map(|v| v.to_string()).collect();
        config.market.exchange_pairs = exchange_pairs.clone();

        let RepositoriesPrepared {
            index_price_repository: _,
            pair_average_price_repositories: _,
            market_repositories,
            percent_change_holder,
            ws_channels_holder,
            index_price,
            pair_average_price,
        } = RepositoriesPrepared::make(&config);

        let markets = configure(
            market_names.clone(),
            exchange_pairs,
            channels,
            1,
            market_repositories,
            pair_average_price,
            index_price,
            config.market.index_pairs,
            &percent_change_holder,
            config.service.percent_change_interval_sec,
            &ws_channels_holder,
            graceful_shutdown,
        )
        .await;

        assert_eq!(market_names.len(), markets.len());

        for (market_name_key, market) in &markets {
            let market_name = market.lock().await.get_spine().name.clone();
            assert_eq!(market_name_key, &market_name);
            assert!(market_names.contains(&market_name.as_str()));
        }
    }

    #[tokio::test]
    async fn test_configure_with_default_params() {
        let config = MarketConfig::default();
        let markets = config.markets.iter().map(|v| v.as_ref()).collect();

        test_configure(markets, config.exchange_pairs, config.channels).await;
    }

    #[tokio::test]
    async fn test_configure_with_custom_params() {
        let markets = vec!["binance", "bitfinex"];
        let coins = vec!["ABC".to_string(), "DEF".to_string(), "GHI".to_string()];
        let exchange_pairs = make_exchange_pairs(coins, Some(vec!["JKL"]));
        let channels = vec![ExternalMarketChannels::Ticker];

        test_configure(markets, exchange_pairs, channels).await;
    }

    #[tokio::test]
    #[should_panic]
    async fn test_configure_panic() {
        let config = MarketConfig::default();
        let markets = vec!["not_existing_market"];

        test_configure(markets, config.exchange_pairs, config.channels).await;
    }
}
