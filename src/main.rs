use crate::worker::worker::Worker;
use std::sync::mpsc;

#[macro_use]
extern crate clap;
use clap::App;

mod worker;

fn main() {
    let yaml = load_yaml!("../resources/cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let markets: Option<Vec<&str>> = matches.values_of("market").map(|v| v.into_iter().collect());
    let coins: Option<Vec<&str>> = matches.values_of("coin").map(|v| v.into_iter().collect());

    let (tx, rx) = mpsc::channel();
    let worker = Worker::new(tx);
    worker.lock().unwrap().start(markets, coins);

    for received_thread in rx {
        let _ = received_thread.join();
    }
}
