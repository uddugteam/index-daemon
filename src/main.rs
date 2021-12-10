use crate::worker::worker::Worker;
use std::sync::mpsc;

#[macro_use]
extern crate clap;
use clap::App;

mod worker;

#[cfg(test)]
mod tests;

// TODO: Implement
fn daemonize() -> bool {
    println!("daemonize called.");

    true
}

fn main() {
    let yaml = load_yaml!("../resources/cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let daemon_mode = if matches.is_present("nodaemon") {
        println!("daemonize NOT called.");

        false
    } else {
        daemonize()
    };

    let main_config_file_path = matches.value_of("main-config").map(|v| v.to_string());
    let coins_config_file_path = matches.value_of("coins-config").map(|v| v.to_string());
    let markets: Option<Vec<&str>> = matches.values_of("market").map(|v| v.into_iter().collect());

    let (tx, rx) = mpsc::channel();
    let worker = Worker::new(main_config_file_path, coins_config_file_path, tx);
    worker.lock().unwrap().start(markets, daemon_mode);

    for received_thread in rx {
        let _ = received_thread.join();
    }
}
