use crate::config_scheme::config_scheme::ConfigScheme;
use crate::graceful_shutdown::start_graceful_shutdown_listener;
use crate::repository::repositories::Repositories;
use crate::worker::defaults::get_pair_average_price_dummy_pair;
use crate::worker::market_helpers::market_value::MarketValue;
use crate::worker::market_helpers::stored_and_ws_transmissible_f64_by_pair_tuple::StoredAndWsTransmissibleF64ByPairTuple;
use crate::worker::network_helpers::ws_server::ws_channel_name::WsChannelName;
use crate::worker::network_helpers::ws_server::ws_channels_holder::make_ws_channels_holder;
use crate::worker::worker::Worker;
use std::sync::{mpsc, Arc, Mutex};

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

mod config_scheme;
mod graceful_shutdown;
mod repository;
mod worker;

#[cfg(test)]
mod test;

fn main() {
    let graceful_shutdown = start_graceful_shutdown_listener();
    let config = ConfigScheme::new();
    let (pair_average_price_repository, market_repositories) =
        Repositories::optionize_fields(Repositories::new(&config));

    let ws_channels_holder = make_ws_channels_holder(&config.market);

    let pair_average_price_ws_channels_key = (
        "worker".to_string(),
        MarketValue::PairAveragePrice,
        get_pair_average_price_dummy_pair(),
    );
    let pair_average_price_ws_channels = ws_channels_holder
        .get(&pair_average_price_ws_channels_key)
        .unwrap();

    let pair_average_price = Arc::new(Mutex::new(StoredAndWsTransmissibleF64ByPairTuple::new(
        pair_average_price_repository.clone(),
        vec![
            WsChannelName::CoinAveragePrice,
            WsChannelName::CoinAveragePriceCandles,
        ],
        None,
        Arc::clone(pair_average_price_ws_channels),
    )));

    let (tx, rx) = mpsc::channel();
    let mut worker = Worker::new(tx, graceful_shutdown);
    worker.start(
        config,
        market_repositories,
        pair_average_price,
        pair_average_price_repository,
        ws_channels_holder,
    );

    for received_thread in rx {
        let _ = received_thread.join();
    }
}
