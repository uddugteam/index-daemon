use libc::c_int;
use signal_hook::{
    consts::signal::{SIGINT, SIGQUIT, SIGTERM},
    iterator::Signals,
    low_level,
};
use std::sync::{Arc, Mutex};
use std::thread;

const SIGNALS: &[c_int] = &[SIGINT, SIGQUIT, SIGTERM];

fn start_force_shutdown_listener() {
    let thread_name = "fn: start_force_shutdown_listener".to_string();
    // Do not join
    let _ = thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            for signal in &mut Signals::new(SIGNALS).unwrap() {
                println!("Force stopping...");
                low_level::emulate_default_handler(signal).unwrap();
            }
        })
        .unwrap();
}

pub fn start_graceful_shutdown_listener() -> Arc<Mutex<bool>> {
    let graceful_shutdown = Arc::new(Mutex::new(false));
    let graceful_shutdown_2 = Arc::clone(&graceful_shutdown);

    let thread_name = "fn: start_graceful_shutdown_listener".to_string();
    // Do not join
    let _ = thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            if (&mut Signals::new(SIGNALS).unwrap())
                .into_iter()
                .next()
                .is_some()
            {
                println!("Gracefully stopping... (press Ctrl+C again to force)");
                start_force_shutdown_listener();

                *graceful_shutdown_2.lock().unwrap() = true;
            }
        })
        .unwrap();

    graceful_shutdown
}
