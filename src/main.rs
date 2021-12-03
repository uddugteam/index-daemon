use crate::worker::worker::Worker;
use std::env;
use std::sync::mpsc;

mod worker;

#[cfg(test)]
mod tests;

const VERSION: &str = "1.0";

// TODO: Implement
fn daemonize() -> bool {
    println!("daemonize called.");

    true
}

fn main() {
    let mut args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "-v" {
        println!("Current version {}", VERSION);

        return;
    }

    if args.len() > 1 && args[1] == "-h" {
        println!("-v Current version");
        println!("-h Prints this message");
        println!("-nodaemon Start with no daemon interface");
        println!("-f=path/to/config Start with custom config-file location for ex. -f=../resources/curr_daemon1.config");
        println!("enter market name(for ex. curr_daemon nodaemon kraken OR curr_daemon kraken) or \"all\" to start daemon on single market or on all markets(default = all)");

        return;
    }

    let daemon_mode = match args.iter().position(|s| s == "-nodaemon") {
        Some(index) => {
            let _ = args.remove(index);
            println!("daemonize NOT called.");

            false
        }
        _ => daemonize(),
    };

    let mut config = None;
    for (index, arg) in args.iter().enumerate() {
        if &arg[0..2] == "-f" {
            config = Some(String::from(&arg[3..]));
            args.remove(index);
            break;
        }
    }

    let (tx, rx) = mpsc::channel();

    let worker = Worker::new(config, tx);

    if args.contains(&"all".to_string()) || args.len() == 1 {
        worker.lock().unwrap().start("all", daemon_mode);
    } else {
        worker.lock().unwrap().start(&args[1], daemon_mode);
    };

    for received_thread in rx {
        let _ = received_thread.join();
    }
}
