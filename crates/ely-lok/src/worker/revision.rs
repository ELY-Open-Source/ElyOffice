//! Each document's revision: requests that may change it count.

use std::{collections::HashMap, io, sync::Mutex};

use crate::ipc::{self, Pack, Unpack, tag};

fn revisions() -> &'static Mutex<HashMap<u64, u64>> {
    static REVISIONS: std::sync::OnceLock<Mutex<HashMap<u64, u64>>> = std::sync::OnceLock::new();
    REVISIONS.get_or_init(Default::default)
}

/// A new session number: this process's prefix, then a count.
pub fn session() -> u64 {
    static PREFIX: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    // Another worker's numbers match nothing here.
    let prefix = *PREFIX.get_or_init(|| {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |at| at.subsec_nanos());
        (u64::from(std::process::id()) ^ u64::from(nanos)) & 0xffff_ffff
    });
    let id = (prefix << 32) | NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    revisions().lock().expect("the revisions lock").insert(id, 0);
    id
}

pub fn forget(id: u64) {
    revisions().lock().expect("the revisions lock").remove(&id);
}

pub fn of(id: u64) -> io::Result<u64> {
    let revisions = revisions().lock().expect("the revisions lock");
    revisions.get(&id).copied().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no such document"))
}

/// The revision, tile generation and blindness now, answered.
pub fn current(unpack: &mut Unpack) -> io::Result<Vec<u8>> {
    let id = unpack.u64()?;
    let revision = of(id)?;
    let (at, clip) = super::with_doc(id, |doc| {
        // A sheet's view paints past its data area.
        let clip = (doc.kind() != 1).then(|| doc.size()).map(|(w, h)| (0, 0, w as i32, h as i32));
        ((doc.part(), doc.edit_mode()), clip)
    })?;
    let blind = crate::events::blind(id, at, clip);
    Ok(Pack::new().u64(revision).u64(crate::events::generation(id)).u64(u64::from(blind)).bytes())
}

/// Refuses unless the document is at the revision asked for.
pub fn expect(unpack: &mut Unpack) -> io::Result<Vec<u8>> {
    let (id, expected) = (unpack.u64()?, unpack.u64()?);
    let now = of(id)?;
    match now == expected {
        true => Ok(Vec::new()),
        false => Err(io::Error::other(format!("{}document {id} is at {now}, not {expected}", ipc::STALE))),
    }
}

/// Before `request` runs: input and commands may change the document.
pub fn bump(request: u8, payload: &[u8]) {
    let changes = matches!(
        request,
        tag::POST_KEY | tag::TURN | tag::POST_MOUSE | tag::GRAPHIC | tag::POST_TEXT | tag::POST_UNO | tag::PASTE | tag::CUT | tag::COMPLETE_FUNCTION
    );
    let mut unpack = Unpack::new(payload);
    let Ok(id) = unpack.u64() else {
        return;
    };
    if !changes || request == tag::POST_MOUSE && hover(&mut unpack) {
        return;
    }
    // The colour scheme touches neither the document nor the selection.
    if request == tag::POST_UNO && unpack.str().is_ok_and(|id| id == "view.theme") {
        return;
    }
    if let Some(revision) = revisions().lock().expect("the revisions lock").get_mut(&id) {
        *revision += 1;
    }
}

/// A move with no button held changes nothing.
fn hover(unpack: &mut Unpack) -> bool {
    // Kind, x, y, count, buttons; kind 2 is a move.
    let fields: Result<Vec<i64>, _> = (0..5).map(|_| unpack.i64()).collect();
    fields.is_ok_and(|fields| fields[0] == 2 && fields[4] == 0)
}
