use std::env;

mod other_helpers;
mod worker;

#[cfg(test)]
mod tests;

const VERSION: &str = "1.0";
static mut DAEMON_MODE: bool = false;

// TODO: Implement
fn daemonize() {
    println!("daemonize called.");

    unsafe {
        DAEMON_MODE = true;
    }
}

fn main() {
    let mut args: Vec<_> = env::args().collect();

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

    match args.iter().position(|s| s == "-nodaemon") {
        Some(index) => {
            let _ = args.remove(index);
            println!("daemonize NOT called.");
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

    let mut worker = worker::Worker::new();
    if args.contains(&String::from("all")) || args.len() == 1 {
        unsafe {
            worker.start("all", DAEMON_MODE, config);
        }
    } else {
        unsafe {
            worker.start(&args[1], DAEMON_MODE, config);
        }
    }
}
