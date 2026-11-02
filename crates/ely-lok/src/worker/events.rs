//! Event batches pushed to the client (LOK-03).

use super::{OUT, quit};
use crate::events::{Events, Rect};
use crate::ipc::{self, Pack, tag};

/// Pushes each document's new events as they come (LOK-03).
pub(super) fn push() {
    loop {
        crate::events::wait_unpushed();
        // Drained under the output lock: current at send.
        let out = OUT.get().expect("the output is set");
        let mut out = out.lock().expect("the output lock");
        for (id, batch) in crate::events::unpushed() {
            let revision = super::revision::of(id).unwrap_or(0);
            let body = [Pack::new().u64(id).u64(revision).bytes(), pack_events(batch)].concat();
            if let Err(error) = ipc::write_frame(&mut *out, tag::EVENTS, ipc::UNASKED, &body) {
                log::warn!("lok-worker: event write failed: {error}");
                quit(1);
            }
        }
    }
}

/// One event batch on the wire.
fn pack_events(batch: Events) -> Vec<u8> {
    let rects = |pack: Pack, rects: &[Rect]| {
        rects
            .iter()
            .fold(pack.u64(rects.len() as u64), |pack, &(x, y, w, h)| {
                pack.i64(x as i64).i64(y as i64).i64(w as i64).i64(h as i64)
            })
    };
    let pack = Pack::new()
        .u64(batch.seq)
        .u64(batch.generation)
        .u64(batch.all as u64);
    let pack = rects(pack, &batch.rects);
    let pack = rects(pack, batch.cursor.as_slice());
    let pack = pack.u64(batch.cursor_visible as u64).str(&batch.pointer);
    let pack = rects(pack, &batch.selection);
    let pack = rects(pack, batch.graphic.as_slice()).u64(batch.cropping as u64);
    let pack = pack
        .u64(batch.review)
        .u64(batch.searched)
        .u64(batch.resized)
        .u64(batch.dismissed);
    let pack = rects(pack, &batch.found);
    let pack = pack.str(batch.menu.as_deref().unwrap_or_default());
    let pack = batch
        .faults
        .iter()
        .fold(pack.u64(batch.faults.len() as u64), |pack, fault| {
            pack.str(fault)
        });
    let pack = batch
        .fonts
        .iter()
        .fold(pack.u64(batch.fonts.len() as u64), |pack, font| {
            pack.str(font)
        });
    let pack = batch.states.iter().fold(
        pack.u64(batch.states.len() as u64),
        |pack, (name, state)| pack.str(name).str(state),
    );
    let pack = batch
        .blocked
        .iter()
        .fold(pack.u64(batch.blocked.len() as u64), |pack, (title, body)| {
            pack.str(title).str(body)
        });
    let pack = batch
        .links
        .iter()
        .fold(pack.u64(batch.links.len() as u64), |pack, link| pack.str(link));
    let pack = batch
        .tabs
        .iter()
        .fold(pack.u64(batch.tabs.len() as u64), |pack, tab| {
            pack.i64(*tab as i64)
        });
    let a11y = &batch.a11y;
    let signed = |at: Option<usize>| at.map_or(-1, |at| at as i64);
    let (start, end) = a11y.selection.unzip();
    let pack = pack
        .str(&a11y.text)
        .i64(signed(a11y.caret))
        .i64(signed(start))
        .i64(signed(end));
    let pack = crate::table::Table::pack(batch.table.as_ref(), batch.cell.pack(pack));
    batch.text.pack(pack).bytes()
}
