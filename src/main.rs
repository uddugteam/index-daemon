use crate::config_scheme::config_scheme::ConfigScheme;
use crate::graceful_shutdown::start_graceful_shutdown_listener;
use crate::helper_functions::fill_historical_data;
use crate::worker::worker::start_worker;
use std::sync::mpsc;

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

mod config_scheme;
mod graceful_shutdown;
mod helper_functions;
mod repository;
#[cfg(test)]
mod test;
mod worker;

fn main() {
    let graceful_shutdown = start_graceful_shutdown_listener();
    let config = ConfigScheme::new();

    fill_historical_data(&config);

    let (tx, rx) = mpsc::channel();
    start_worker(config, tx, graceful_shutdown);

    for received_thread in rx {
        let _ = received_thread.join();
    }
}
