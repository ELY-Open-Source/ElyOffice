//! Event batches the worker pushes, kept per document (LOK-03).

use std::{
    collections::{HashMap, HashSet},
    io,
    sync::{Arc, Condvar, Mutex},
    task::{Poll, Waker},
    time::{Duration, Instant},
};

use super::Doc;
use crate::events::Events;

/// The longest event wait in milliseconds.
const MAX_WAIT_MS: u64 = 1000;

#[derive(Default)]
struct Docs {
    /// The last snapshot, and the batch not yet taken.
    open: HashMap<u64, (Events, Option<Events>)>,
    closed: HashSet<u64>,
    dead: bool,
    /// Async waiters to wake on any change.
    wakers: Vec<Waker>,
}

/// Waits on pushed events without the worker lock.
#[derive(Clone, Default)]
pub struct Feed(Arc<(Mutex<Docs>, Condvar)>);

/// `next` with the one-shot parts of `before` kept.
fn merge(before: Events, mut next: Events) -> Events {
    next.all |= before.all;
    let mut rects = before.rects;
    rects.append(&mut next.rects);
    next.rects = rects;
    let mut states = before.states;
    states.retain(|(name, _)| !next.states.iter().any(|(newer, _)| newer == name));
    states.append(&mut next.states);
    next.states = states;
    let mut blocked = before.blocked;
    blocked.append(&mut next.blocked);
    next.blocked = blocked;
    let mut links = before.links;
    links.append(&mut next.links);
    next.links = links;
    next.menu = next.menu.or(before.menu);
    let mut faults = before.faults;
    faults.append(&mut next.faults);
    next.faults = faults;
    next
}

/// A snapshot with the one-shot parts cleared.
fn snapshot(batch: &Events) -> Events {
    Events {
        all: false,
        rects: Vec::new(),
        states: Vec::new(),
        blocked: Vec::new(),
        links: Vec::new(),
        menu: None,
        faults: Vec::new(),
        ..batch.clone()
    }
}

impl Feed {
    fn docs(&self) -> std::sync::MutexGuard<'_, Docs> {
        self.0.0.lock().expect("the feed lock")
    }

    pub(crate) fn opened(&self, id: u64) {
        self.docs().open.entry(id).or_default();
    }

    pub(crate) fn closed(&self, id: u64) {
        let mut docs = self.docs();
        docs.open.remove(&id);
        docs.closed.insert(id);
        self.wake(docs);
    }

    pub(crate) fn push(&self, id: u64, batch: Events) {
        let mut docs = self.docs();
        if docs.closed.contains(&id) {
            return;
        }
        let (last, fresh) = docs.open.entry(id).or_default();
        *last = snapshot(&batch);
        *fresh = Some(match fresh.take() {
            Some(before) => merge(before, batch),
            None => batch,
        });
        self.wake(docs);
    }

    pub(crate) fn died(&self) {
        let mut docs = self.docs();
        docs.dead = true;
        self.wake(docs);
    }

    /// Wakes blocked and async waiters.
    fn wake(&self, mut docs: std::sync::MutexGuard<'_, Docs>) {
        let wakers = std::mem::take(&mut docs.wakers);
        drop(docs);
        self.0.1.notify_all();
        wakers.into_iter().for_each(Waker::wake);
    }

    /// Events after `known`, awaited without holding a thread.
    pub async fn next(&self, doc: &Doc, known: u64) -> io::Result<Events> {
        std::future::poll_fn(|cx| {
            let mut docs = self.docs();
            if docs.dead {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "the lok worker is gone",
                )));
            }
            let Some((_, fresh)) = docs.open.get_mut(&doc.id) else {
                return Poll::Ready(Err(io::Error::other("no such document")));
            };
            if fresh.as_ref().is_some_and(|batch| batch.seq > known) {
                return Poll::Ready(Ok(fresh.take().expect("checked above")));
            }
            docs.wakers.push(cx.waker().clone());
            Poll::Pending
        })
        .await
    }

    /// Events after `known`, or the snapshot after `timeout_ms`.
    pub fn wait(&self, doc: &Doc, known: u64, timeout_ms: u64) -> io::Result<Events> {
        let id = doc.id;
        let deadline = Instant::now() + Duration::from_millis(timeout_ms.min(MAX_WAIT_MS));
        let mut docs = self.docs();
        loop {
            if docs.dead {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "the lok worker is gone",
                ));
            }
            let Some((last, fresh)) = docs.open.get_mut(&id) else {
                return Err(io::Error::other("no such document"));
            };
            if fresh.as_ref().is_some_and(|batch| batch.seq > known) {
                return Ok(fresh.take().expect("checked above"));
            }
            let now = Instant::now();
            if now >= deadline {
                return Ok(last.clone());
            }
            docs = self
                .0
                .1
                .wait_timeout(docs, deadline - now)
                .expect("the feed wait")
                .0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queued_batches_keep_their_one_shot_parts() {
        let feed = Feed::default();
        feed.opened(1);
        let first = Events {
            seq: 1,
            rects: vec![(0, 0, 1, 1)],
            states: vec![("a".into(), "1".into()), ("b".into(), "1".into())],
            ..Events::default()
        };
        let second = Events {
            seq: 2,
            all: true,
            states: vec![("a".into(), "2".into())],
            ..Events::default()
        };
        feed.push(1, first);
        feed.push(1, second);
        let batch = feed.wait(&Doc { worker: 0, id: 1, rev: 0, kind: 0 }, 0, 0).expect("a batch");
        assert_eq!((batch.seq, batch.all, batch.rects.len()), (2, true, 1));
        assert_eq!(
            batch.states,
            [("b".to_string(), "1".to_string()), ("a".into(), "2".into())]
        );
        let idle = feed.wait(&Doc { worker: 0, id: 1, rev: 0, kind: 0 }, 2, 0).expect("the snapshot");
        assert!(idle.rects.is_empty() && idle.states.is_empty() && idle.seq == 2);
        feed.closed(1);
        assert!(
            feed.wait(&Doc { worker: 0, id: 1, rev: 0, kind: 0 }, 2, 0).is_err(),
            "a closed document"
        );
    }
}
