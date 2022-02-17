use crate::config_scheme::config_scheme::ConfigScheme;
use crate::config_scheme::repositories_prepared::RepositoriesPrepared;
use crate::graceful_shutdown::start_graceful_shutdown_listener;
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

    let RepositoriesPrepared {
        pair_average_price_repository,
        market_repositories,
        ws_channels_holder,
        pair_average_price,
    } = RepositoriesPrepared::make(&config);

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
