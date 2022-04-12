use crate::config_scheme::config_scheme::ConfigScheme;
use crate::graceful_shutdown::GracefulShutdown;
use crate::helper_functions::fill_historical_data;
use crate::worker::worker::start_worker;

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

#[tokio::main]
async fn main() {
    let graceful_shutdown = GracefulShutdown::new();
    let graceful_shutdown_future = graceful_shutdown.clone().start_listener();
    let config = ConfigScheme::new();

    fill_historical_data(&config).await;

    let worker_future = start_worker(config, graceful_shutdown);

    tokio::select! {
        _ = graceful_shutdown_future => {},
        _ = worker_future => {},
    };
}
