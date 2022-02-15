use crate::config_scheme::config_scheme::ConfigScheme;
use crate::graceful_shutdown::start_graceful_shutdown_listener;
use crate::repository::repositories::Repositories;
use crate::worker::market_helpers::pair_average_price::make_pair_average_price;
use crate::worker::network_helpers::ws_server::ws_channels_holder::WsChannelsHolder;
use crate::worker::worker::Worker;
use std::sync::mpsc;

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

mod config_scheme;
mod graceful_shutdown;
mod repository;
#[cfg(test)]
mod test;
mod worker;

fn main() {
    let graceful_shutdown = start_graceful_shutdown_listener();
    let config = ConfigScheme::new();
    let (pair_average_price_repository, market_repositories) =
        Repositories::optionize_fields(Repositories::new(&config));

    let ws_channels_holder = WsChannelsHolder::make_hashmap(&config.market);

    let pair_average_price = make_pair_average_price(
        &config.market,
        pair_average_price_repository.clone(),
        &ws_channels_holder,
    );

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
