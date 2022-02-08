use crate::config_scheme::config_scheme::ConfigScheme;
use crate::graceful_shutdown::start_graceful_shutdown_listener;
use crate::repository::repositories::Repositories;
use crate::worker::worker::Worker;
use std::sync::mpsc;

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
    let (pair_average_price, pair_average_price_historical, market_repositories) =
        Repositories::optionize_fields(Repositories::new(&config));

    let (tx, rx) = mpsc::channel();
    let worker = Worker::new(tx, graceful_shutdown, pair_average_price);
    worker
        .lock()
        .unwrap()
        .start(config, market_repositories, pair_average_price_historical);

    for received_thread in rx {
        let _ = received_thread.join();
    }
}
