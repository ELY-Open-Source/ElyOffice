//! Ordered writes of one file from many threads.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

/// Numbers writes of one file; only the newest lands.
#[derive(Clone, Default)]
pub struct Turns(Arc<Inner>);

#[derive(Default)]
struct Inner {
    issued: AtomicU64,
    writing: Mutex<()>,
}

impl Turns {
    /// The next turn; never waits on a write.
    pub fn take(&self) -> u64 {
        self.0.issued.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Runs `write` unless a later turn came.
    pub fn write<T>(&self, turn: u64, write: impl FnOnce() -> T) -> Option<T> {
        let _writing = self.0.writing.lock().expect("the write lock");
        (self.0.issued.load(Ordering::SeqCst) == turn).then(write)
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, time::Duration};

    /// Takes a turn elsewhere; fails if it waits.
    fn take_within(turns: &super::Turns) -> u64 {
        let (taken, took) = mpsc::channel();
        let taker = turns.clone();
        std::thread::spawn(move || taken.send(taker.take()).expect("sends"));
        took.recv_timeout(Duration::from_secs(2)).expect("taking a turn waited on a write")
    }

    #[test]
    fn a_later_turn_supersedes_an_earlier() {
        let turns = super::Turns::default();
        let (early, late) = (turns.take(), turns.take());
        assert_eq!(turns.write(early, || "old"), None, "the early write skips");
        assert_eq!(turns.write(late, || "new"), Some("new"));
    }

    #[test]
    fn taking_a_turn_never_waits_on_a_write() {
        let turns = super::Turns::default();
        let turn = turns.take();
        let (entered, release) = (mpsc::channel(), mpsc::channel::<()>());
        let writer = {
            let turns = turns.clone();
            std::thread::spawn(move || {
                turns.write(turn, || {
                    entered.0.send(()).expect("says it writes");
                    release.1.recv().expect("released");
                })
            })
        };
        entered.1.recv().expect("the write holds its lock");
        assert_eq!(take_within(&turns), turn + 1);
        release.0.send(()).expect("releases");
        writer.join().expect("the writer ends");
    }

    #[test]
    fn a_write_waits_for_the_one_in_flight() {
        let turns = super::Turns::default();
        let (early, entered, release) = (turns.take(), mpsc::channel(), mpsc::channel::<()>());
        let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let writer = {
            let (turns, order) = (turns.clone(), order.clone());
            std::thread::spawn(move || {
                turns.write(early, || {
                    entered.0.send(()).expect("says it writes");
                    release.1.recv().expect("released");
                    order.lock().expect("order").push("early");
                })
            })
        };
        entered.1.recv().expect("the early write runs");
        let late = take_within(&turns);
        let lands = {
            let (turns, order) = (turns.clone(), order.clone());
            std::thread::spawn(move || turns.write(late, || order.lock().expect("order").push("late")))
        };
        std::thread::sleep(Duration::from_millis(200));
        assert!(order.lock().expect("order").is_empty(), "the late write waits");
        release.0.send(()).expect("releases");
        writer.join().expect("the early ends");
        lands.join().expect("the late ends").expect("the newest lands");
        assert_eq!(*order.lock().expect("order"), ["early", "late"]);
    }
}
