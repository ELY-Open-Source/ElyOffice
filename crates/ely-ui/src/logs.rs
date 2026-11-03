//! What an app logs, kept for tests to read.

use std::sync::{Mutex, Once};

static KEPT: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct Kept;

impl log::Log for Kept {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!("{} {} {}", record.level(), record.target(), record.args());
        // A failing test prints its log.
        println!("{line}");
        KEPT.lock().expect("the kept log").push(line);
    }

    fn flush(&self) {}
}

/// Keeps every record from now on, once per process.
pub fn keep() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        log::set_boxed_logger(Box::new(Kept)).expect("no other logger in tests");
        log::set_max_level(log::LevelFilter::Info);
    });
}

/// Every record kept so far.
pub fn kept() -> Vec<String> {
    KEPT.lock().expect("the kept log").clone()
}
