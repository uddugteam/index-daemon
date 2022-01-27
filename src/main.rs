use std::sync::mpsc;

use crate::config_scheme::config_scheme::ConfigScheme;
use crate::graceful_shutdown::start_graceful_shutdown_listener;
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

    let (tx, rx) = mpsc::channel();
    let worker = Worker::new(tx, graceful_shutdown);
    worker.lock().unwrap().start(config);

    for received_thread in rx {
        let _ = received_thread.join();
    }
}
