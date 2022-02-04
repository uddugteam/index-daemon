use std::sync::mpsc;

use crate::config_scheme::config_scheme::ConfigScheme;
use crate::graceful_shutdown::start_graceful_shutdown_listener;
use crate::repository::repositories::Repositories;
use crate::worker::worker::Worker;

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
    let repositories = Repositories::new(&config.market);

    let (tx, rx) = mpsc::channel();
    let worker = Worker::new(tx, graceful_shutdown, repositories.pair_average_price);
    worker
        .lock()
        .unwrap()
        .start(config, repositories.market_repositories);

    for received_thread in rx {
        let _ = received_thread.join();
    }
}
