use crate::config_scheme::config_scheme::ConfigScheme;
use crate::config_scheme::market_config::MarketConfig;
use crate::config_scheme::repositories_prepared::RepositoriesPrepared;
use crate::config_scheme::service_config::ServiceConfig;
use crate::repository::repositories::{
    MarketRepositoriesByMarketName, RepositoryForF64ByTimestamp, WorkerRepositoriesByPairTuple,
};
use crate::worker::db_cleaner::clear_db;
use crate::worker::market_helpers::market::{market_factory, market_update, Market};
use crate::worker::market_helpers::market_channels::MarketChannels;
use crate::worker::market_helpers::market_spine::MarketSpine;
use crate::worker::market_helpers::pair_average_price::StoredAndWsTransmissibleF64ByPairTuple;
use crate::worker::market_helpers::percent_change::PercentChangeByInterval;
use crate::worker::market_helpers::stored_and_ws_transmissible_f64::StoredAndWsTransmissibleF64;
use crate::worker::network_helpers::ws_server::holders::helper_functions::HolderHashMap;
use crate::worker::network_helpers::ws_server::holders::percent_change_holder::PercentChangeByIntervalHolder;
use crate::worker::network_helpers::ws_server::holders::ws_channels_holder::WsChannelsHolder;
use crate::worker::network_helpers::ws_server::ws_channels::WsChannels;
use crate::worker::network_helpers::ws_server::ws_server::WsServer;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time;

fn is_graceful_shutdown(graceful_shutdown: &Arc<RwLock<bool>>) -> bool {
    *graceful_shutdown.read().unwrap()
}

fn configure(
    market_names: Vec<&str>,
    exchange_pairs: Vec<(String, String)>,
    channels: Vec<MarketChannels>,
    rest_timeout_sec: u64,
    repositories: Option<MarketRepositoriesByMarketName>,
    pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
    index_price: Arc<RwLock<StoredAndWsTransmissibleF64>>,
    index_pairs: Vec<(String, String)>,
    percent_change_holder: &HolderHashMap<PercentChangeByInterval>,
    percent_change_interval_sec: u64,
    ws_channels_holder: &HolderHashMap<WsChannels>,
    tx: Sender<JoinHandle<()>>,
    graceful_shutdown: Arc<RwLock<bool>>,
) -> HashMap<String, Arc<Mutex<dyn Market + Send>>> {
    let mut markets = HashMap::new();
    let mut repositories = repositories.unwrap_or_default();

    for market_name in market_names {
        let pair_average_price_2 = pair_average_price.clone();
        let market_spine = MarketSpine::new(
            pair_average_price_2,
            Arc::clone(&index_price),
            index_pairs.clone(),
            tx.clone(),
            rest_timeout_sec,
            market_name.to_string(),
            channels.clone(),
            Arc::clone(&graceful_shutdown),
        );
        let market = market_factory(
            market_spine,
            exchange_pairs.clone(),
            repositories.remove(market_name),
            percent_change_holder,
            percent_change_interval_sec,
            ws_channels_holder,
        );

        markets.insert(market_name.to_string(), market);
    }

    markets
}

fn start_ws(
    ws: bool,
    ws_addr: String,
    ws_answer_timeout_ms: u64,
    index_price_repository: Option<RepositoryForF64ByTimestamp>,
    pair_average_price: StoredAndWsTransmissibleF64ByPairTuple,
    percent_change_holder: HolderHashMap<PercentChangeByInterval>,
    ws_channels_holder: HolderHashMap<WsChannels>,
    tx: Sender<JoinHandle<()>>,
    graceful_shutdown: Arc<RwLock<bool>>,
) {
    if ws {
        let percent_change_holder = PercentChangeByIntervalHolder::new(percent_change_holder);
        let ws_channels_holder = WsChannelsHolder::new(ws_channels_holder);

        let thread_name = "fn: start_ws".to_string();
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                let ws_server = WsServer {
                    percent_change_holder,
                    ws_channels_holder,
                    ws_addr,
                    ws_answer_timeout_ms,
                    index_price_repository,
                    pair_average_price,
                    graceful_shutdown,
                };
                ws_server.start();
            })
            .unwrap();
        tx.send(thread).unwrap();
    }
}

fn start_db_cleaner(
    index_price_repository: Option<RepositoryForF64ByTimestamp>,
    pair_average_price_repositories: Option<WorkerRepositoriesByPairTuple>,
    market_repositories: Option<MarketRepositoriesByMarketName>,
    data_expire_sec: u64,
    tx: Sender<JoinHandle<()>>,
) {
    let thread_name = "fn: clear_db".to_string();
    let thread = thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            loop {
                clear_db(
                    index_price_repository.clone(),
                    pair_average_price_repositories.clone(),
                    market_repositories.clone(),
                    data_expire_sec,
                );

                // Sleep 1 day
                thread::sleep(time::Duration::from_secs(86_400));
            }
        })
        .unwrap();
    tx.send(thread).unwrap();
}

pub fn start_worker(
    config: ConfigScheme,
    tx: Sender<JoinHandle<()>>,
    graceful_shutdown: Arc<RwLock<bool>>,
) {
    let RepositoriesPrepared {
        index_price_repository,
        pair_average_price_repositories,
        market_repositories,
        percent_change_holder,
        ws_channels_holder,
        pair_average_price,
        index_price,
    } = RepositoriesPrepared::make(&config);

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
        tx.clone(),
        graceful_shutdown.clone(),
    );
    start_ws(
        ws,
        ws_addr,
        ws_answer_timeout_ms,
        index_price_repository.clone(),
        pair_average_price,
        percent_change_holder,
        ws_channels_holder,
        tx.clone(),
        graceful_shutdown.clone(),
    );

    start_db_cleaner(
        index_price_repository,
        pair_average_price_repositories,
        market_repositories,
        data_expire_sec,
        tx.clone(),
    );

    for (_market_name, market) in markets {
        if is_graceful_shutdown(&graceful_shutdown) {
            return;
        }

        let thread_name = format!(
            "fn: market_update, market: {}",
            market.lock().unwrap().get_spine().name
        );
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                market_update(market);
            })
            .unwrap();
        tx.send(thread).unwrap();
    }
}

#[cfg(test)]
pub mod test {
    use crate::config_scheme::config_scheme::ConfigScheme;
    use crate::config_scheme::helper_functions::make_exchange_pairs;
    use crate::config_scheme::market_config::MarketConfig;
    use crate::config_scheme::repositories_prepared::RepositoriesPrepared;
    use crate::worker::market_helpers::market_channels::MarketChannels;
    use crate::worker::worker::Worker;
    use ntest::timeout;
    use serial_test::serial;
    use std::sync::mpsc::{Receiver, Sender};
    use std::sync::{mpsc, Arc, RwLock};
    use std::thread;
    use std::thread::JoinHandle;
    use std::time;

    pub fn make_worker() -> (Worker, Sender<JoinHandle<()>>, Receiver<JoinHandle<()>>) {
        let (tx, rx) = mpsc::channel();
        let graceful_shutdown = Arc::new(RwLock::new(false));
        let worker = Worker::new(tx.clone(), graceful_shutdown);

        (worker, tx, rx)
    }

    pub fn check_threads(mut thread_names: Vec<String>, rx: Receiver<JoinHandle<()>>) {
        let mut passed_thread_names = Vec::new();
        for received_thread in rx {
            let thread_name = received_thread.thread().name().unwrap().to_string();
            assert!(!passed_thread_names.contains(&thread_name));

            if let Some(index) = thread_names.iter().position(|r| r == &thread_name) {
                passed_thread_names.push(thread_names.swap_remove(index));
            }
            if thread_names.is_empty() {
                break;
            }
        }
    }

    fn test_configure(
        markets: Vec<&str>,
        exchange_pairs: Vec<(String, String)>,
        channels: Vec<MarketChannels>,
    ) {
        let (mut worker, _, _) = make_worker();

        let mut config = ConfigScheme::default();
        config.market.markets = markets.iter().map(|v| v.to_string()).collect();
        config.market.exchange_pairs = exchange_pairs.clone();

        let RepositoriesPrepared {
            index_price_repository: _,
            pair_average_price_repositories: _,
            market_repositories,
            ws_channels_holder,
            pair_average_price,
            index_price,
        } = RepositoriesPrepared::make(&config);

        worker.configure(
            markets.clone(),
            exchange_pairs,
            channels,
            1,
            market_repositories,
            pair_average_price,
            index_price,
            config.market.index_pairs,
            &ws_channels_holder,
            config.service.percent_change_interval_sec,
        );

        assert_eq!(markets.len(), worker.markets.len());

        for (market_name_key, market) in &worker.markets {
            let market_name = market.lock().unwrap().get_spine().name.clone();
            assert_eq!(market_name_key, &market_name);
            assert!(markets.contains(&market_name.as_str()));
        }
    }

    #[test]
    fn test_configure_with_default_params() {
        let config = MarketConfig::default();
        let markets = config.markets.iter().map(|v| v.as_ref()).collect();

        test_configure(markets, config.exchange_pairs, config.channels);
    }

    #[test]
    fn test_configure_with_custom_params() {
        let markets = vec!["binance", "bitfinex"];
        let coins = vec!["ABC".to_string(), "DEF".to_string(), "GHI".to_string()];
        let exchange_pairs = make_exchange_pairs(coins, Some(vec!["JKL"]));
        let channels = vec![MarketChannels::Ticker];

        test_configure(markets, exchange_pairs, channels);
    }

    #[test]
    #[should_panic]
    fn test_configure_panic() {
        let config = MarketConfig::default();
        let markets = vec!["not_existing_market"];

        test_configure(markets, config.exchange_pairs, config.channels);
    }

    /// TODO: Add tests for WsServer
    fn inner_test_start(config: ConfigScheme) {
        // To prevent DDoS attack on exchanges
        thread::sleep(time::Duration::from_millis(3000));

        let (mut worker, _, rx) = make_worker();

        let mut thread_names = Vec::new();
        for market in &config.market.markets {
            let thread_name = format!("fn: market_update, market: {}", market);
            thread_names.push(thread_name);
        }

        worker.start_worker(config);
        check_threads(thread_names, rx);
    }

    /// `serial` and `timeout` are incompatible
    #[test]
    // #[serial]
    #[timeout(5000)]
    fn test_start_with_default_params() {
        let config = ConfigScheme::default();

        inner_test_start(config);
    }

    /// `serial` and `timeout` are incompatible
    #[test]
    // #[serial]
    #[timeout(5000)]
    fn test_start_with_custom_params() {
        let markets = vec!["binance".to_string(), "bitfinex".to_string()];
        let coins = vec!["ABC".to_string(), "DEF".to_string()];
        let exchange_pairs = make_exchange_pairs(coins, Some(vec!["GHI"]));
        let index_pairs = exchange_pairs.clone();
        let channels = vec![MarketChannels::Ticker];

        let config = ConfigScheme {
            market: MarketConfig {
                markets,
                exchange_pairs,
                index_pairs,
                channels,
            },
            ..ConfigScheme::default()
        };

        inner_test_start(config);
    }

    #[test]
    #[serial]
    #[should_panic]
    fn test_start_panic() {
        let mut config = ConfigScheme::default();
        config.market.markets = vec!["not_existing_market".to_string()];

        inner_test_start(config);
    }
}
