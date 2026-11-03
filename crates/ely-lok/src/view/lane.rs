//! One background lane per view: inputs first, then tiles (LOK-03).

use std::{
    io,
    sync::{Arc, Mutex},
    time::Instant,
};

use futures::{StreamExt, channel::mpsc, select_biased};
use gpui::Context;

use super::clock::Picture;
use super::input::{Back, Input};
use super::{Geom, LokError, LokView, Reported, TILE_TWIPS};
use crate::{Doc, Tile, Worker};

/// An input slower than this is logged as a warning.
const SLOW_MS: u128 = 100;

/// A tile to paint at `key` for `geom`.
pub(super) struct Job {
    pub key: (i32, i32),
    pub geom: Geom,
}

/// What came back from the lane.
enum Landed {
    /// Sequence, whether it edits, whether it switched part, outcome.
    Input(u64, bool, bool, (io::Result<Option<Back>>, Option<u64>, Picture)),
    Tile(Job, io::Result<Tile>),
    /// Off screen or old by the time its turn came.
    Skipped((i32, i32)),
}

/// The lane's two queues.
pub(super) struct Lane {
    pub inputs: mpsc::UnboundedSender<(u64, Input, Instant)>,
    pub tiles: mpsc::UnboundedSender<Job>,
}

impl LokView {
    /// Starts the lane; it ends with the view.
    pub(super) fn lane(worker: Arc<Mutex<Worker>>, doc: Doc, wanted: super::reveal::Wanted, cx: &mut Context<Self>) -> Lane {
        let (inputs, mut input_queue) = mpsc::unbounded::<(u64, Input, Instant)>();
        let (tiles, mut tile_queue) = mpsc::unbounded::<Job>();
        let (landing, mut landed) = mpsc::unbounded::<Landed>();
        cx.background_executor()
            .spawn(async move {
                'lane: loop {
                    let done = select_biased! {
                        input = input_queue.next() => {
                            let Some(first) = input else { break };
                            let mut batch = vec![first];
                            // Queued inputs share one hold of the worker.
                            while let Ok(more) = input_queue.try_recv() {
                                batch.push(more);
                            }
                            let mut held = worker.lock().expect("the worker lock");
                            let mut done = Vec::with_capacity(batch.len());
                            for (seq, input, sent) in batch {
                                let (waited, edits, kind) = (sent.elapsed().as_millis(), input.edits(), input.kind());
                                let switched = matches!(input, super::input::Input::Part(_));
                                let outcome = super::input::run(&mut held, &doc, input);
                                let took = sent.elapsed().as_millis();
                                if took > SLOW_MS {
                                    log::warn!("lok: input {seq} {kind} waited {waited}, took {took} ms");
                                } else {
                                    log::debug!("lok: input {seq} {kind} waited {waited}, took {took} ms");
                                }
                                done.push(Landed::Input(seq, edits, switched, outcome));
                            }
                            done
                        }
                        job = tile_queue.next() => {
                            let Some(job) = job else { break };
                            let still = {
                                let wanted = super::reveal::wanted(&wanted);
                                wanted.0 == job.geom && wanted.1.contains(&job.key)
                            };
                            vec![if !still {
                                log::debug!("lok: tile job {:?} skipped", job.key);
                                Landed::Skipped(job.key)
                            } else {
                                let started = Instant::now();
                                let side = job.geom.tile_pixels();
                                let (x, y) = (job.key.0 * TILE_TWIPS, job.key.1 * TILE_TWIPS);
                                let mut held = worker.lock().expect("the worker lock");
                                let waited = started.elapsed().as_millis();
                                let tile = doc.paint_tile(&mut held, (x, y, TILE_TWIPS, TILE_TWIPS), (side, side));
                                drop(held);
                                let key = job.key;
                                log::debug!("lok: tile {key:?} in {} ms, {waited} waiting for the worker", started.elapsed().as_millis());
                                Landed::Tile(job, tile)
                            }]
                        }
                    };
                    for done in done {
                        if landing.unbounded_send(done).is_err() {
                            break 'lane;
                        }
                    }
                }
            })
            .detach();
        cx.spawn(async move |this, cx| {
            while let Some(first) = landed.next().await {
                let mut batch = vec![first];
                while let Ok(more) = landed.try_recv() {
                    batch.push(more);
                }
                let taken = this.update(cx, |view, cx| view.take_landed(batch, cx));
                if taken.is_err() {
                    break;
                }
            }
        })
        .detach();
        Lane { inputs, tiles }
    }

    fn take_landed(&mut self, batch: Vec<Landed>, cx: &mut Context<Self>) {
        let mut reported = false;
        for landed in batch {
            match landed {
                Landed::Input(seq, edits, switched, outcome) => {
                    self.applied = seq;
                    // Same tile keys, another slide or sheet.
                    if switched {
                        self.invalidate(cx);
                    }
                    self.edited += u64::from(edits);
                    let (outcome, now, picture) = outcome;
                    // A plain key took nothing; the menu may return.
                    let taken = matches!(outcome, Ok(Some(Back::Completed(true))));
                    self.completion.landed(seq, if taken { now } else { None });
                    if edits {
                        self.clock.landed(seq, picture);
                    }
                    self.seen_revision(now.unwrap_or(0));
                    match outcome {
                        Ok(Some(Back::Copied(copied))) => self.copied = Some(copied),
                        Ok(Some(Back::Shown(part))) if part != self.part => {
                            // The worker turned in order.
                            self.part = part;
                            self.invalidate(cx);
                        }
                        Ok(Some(Back::Shown(_) | Back::Completed(_))) => {}
                        Ok(None) => {}
                        Err(error) if crate::is_nothing(&error) => log::info!("lok: nothing to undo or redo"),
                        Err(error) => {
                            log::error!("lok: input failed ({:?})", error.kind());
                            cx.emit(LokError(error.to_string()));
                        }
                    }
                    reported = true;
                }
                Landed::Tile(job, tile) => self.finish_tile(job.key, job.geom, tile, cx),
                Landed::Skipped(key) => {
                    self.fetching.remove(&key);
                }
            }
        }
        if reported {
            cx.emit(Reported);
        }
        cx.notify();
    }
}
