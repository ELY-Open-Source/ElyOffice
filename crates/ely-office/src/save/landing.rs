//! Export and disk write off the UI thread (FILE-04).

use std::{io, path::PathBuf};

use ely_files::{Done, SaveError, Stamp, Write};
use gpui::{AppContext as _, Context, TaskExt as _, Window};

use super::{Saving, gone};
use crate::files::{self, file_name};
use crate::{After, Closing as _, Engine, Risk, Tabs as _, Waiting, in_window};

/// Where an export's bytes go.
pub(super) enum Target {
    /// The open file; an outside change conflicts.
    Own,
    /// Keep Mine over a gone file; never a new one.
    Gone,
    /// Save As here; `true` reopens what was written.
    As(PathBuf, bool),
}

/// What the save's end needs from its start.
struct Landing {
    then: Option<After>,
    /// The view's edit count when the export began.
    edited: u64,
    /// The session's content when the export began.
    stamp: Stamp,
    history: Option<String>,
}

/// Exports `format` in the background, then writes it there too.
pub(super) fn export<H: Saving>(
    host: &mut H,
    id: u64,
    format: &'static str,
    then: Option<After>,
    target: Target,
    cx: &mut Context<H>,
) {
    let Some(tab) = host.tab_mut(id) else {
        return;
    };
    tab.exporting = true;
    let (work, view, stamp) = (tab.work.clone(), tab.view.clone(), tab.session.stamp());
    let edited = H::Engine::edited(view.read(cx));
    // Other saves leave an open edit as it is.
    let closing = then.is_some();
    cx.notify();
    let sound = !matches!(target, Target::As(..));
    host.read_in(
        id,
        move |worker, doc| {
            if sound && let Some(risk) = H::Engine::unsound(worker, doc)? {
                return Ok(Err(risk));
            }
            let bytes = files::kernel_bytes::<H::Engine>(worker, doc, &work, format, closing)?;
            Ok(Ok((bytes, H::Engine::history(worker, doc)?)))
        },
        move |host, read, window, cx| {
            let landing = Landing { then, edited, stamp, history: None };
            match read {
                _ if host.tab(id).is_none() => ended(host, id, Err(closed(id)), landing, window, cx),
                Ok(Ok((bytes, history))) => {
                    let landing = Landing { history: Some(history), ..landing };
                    write(host, id, bytes, target, landing, window, cx);
                }
                Ok(Err(risk)) => risky(host, id, risk, then, window, cx),
                Err(error) => ended(host, id, Err(SaveError::Io(error)), landing, window, cx),
            }
        },
        cx,
    );
}

fn closed(id: u64) -> SaveError {
    SaveError::Io(io::Error::other(format!("tab {id} closed during its save")))
}

/// Starts the save; the disk work runs elsewhere.
fn write<H: Saving>(
    host: &mut H,
    id: u64,
    bytes: Vec<u8>,
    target: Target,
    landing: Landing,
    window: &mut Window,
    cx: &mut Context<H>,
) {
    if let Target::As(path, _) = &target {
        // Another tab may have taken the target meanwhile.
        match host.holder(path) {
            Ok(Some(other)) if other != id => {
                let name = file_name(path);
                let taken = ely_ipc::Worded::error(io::ErrorKind::Other, "office.error.taken", &[("name", &name)]);
                return ended(host, id, Err(SaveError::Io(taken)), landing, window, cx);
            }
            Err(error) => return ended(host, id, Err(SaveError::Io(error)), landing, window, cx),
            _ => {}
        }
    }
    let Some(tab) = host.tab_mut(id) else {
        return ended(host, id, Err(closed(id)), landing, window, cx);
    };
    let begun = match &target {
        Target::Own => Write::save(landing.stamp, &mut tab.session),
        Target::Gone => Write::anew(landing.stamp, &mut tab.session),
        Target::As(path, _) => Write::save_as(landing.stamp, &mut tab.session, path),
    };
    let write = match begun {
        Ok(write) => write,
        Err(error) => return ended(host, id, Err(error), landing, window, cx),
    };
    log::info!("office: tab {id} writes {} bytes off the window", bytes.len());
    let moving = matches!(target, Target::As(..));
    let task = cx.background_spawn(async move {
        let done = write.run(&bytes);
        // A new file's macros are read here.
        let risk = done.saved.as_ref().ok().filter(|_| moving).map(|saved| files::macro_risk(&H::KIND, &saved.path));
        (done, risk)
    });
    cx.spawn(async move |this, cx| {
        let (done, risk) = task.await;
        in_window(&this, cx, |host, window, cx| landed(host, id, (done, risk), target, landing, window, cx))
    })
    .detach_and_log_err(cx);
}

/// The written save reaches its session, then the shell.
fn landed<H: Saving>(
    host: &mut H,
    id: u64,
    (done, risk): (Done, Option<io::Result<bool>>),
    target: Target,
    landing: Landing,
    window: &mut Window,
    cx: &mut Context<H>,
) {
    let Some(tab) = host.tab_mut(id) else {
        return ended(host, id, done.saved, landing, window, cx);
    };
    let saved = done.land(&mut tab.session);
    match (&saved, target, risk) {
        (Ok(saved), Target::As(_, reopen), Some(risk)) => {
            // Taken meanwhile: a failed save, edits kept.
            if let Err(taken) = host.moved(id, saved, risk, cx) {
                return ended(host, id, Err(SaveError::Io(taken)), landing, window, cx);
            }
            if let Some(tab) = host.tab_mut(id) {
                tab.reopen = reopen;
            }
        }
        // Moved or deleted outside: ask where instead.
        (Err(SaveError::Io(error)), Target::Own, _) if error.kind() == io::ErrorKind::NotFound => {
            let path = host.tab(id).and_then(|tab| tab.session.path().map(PathBuf::from));
            let queued = settle(host, id);
            if let Some(path) = path {
                gone::gone(host, id, &path, landing.then, cx);
            }
            return run(host, queued, window, cx);
        }
        _ => {}
    }
    ended(host, id, saved, landing, window, cx)
}

/// Found at the last moment: write elsewhere instead.
fn risky<H: Saving>(host: &mut H, id: u64, risk: Risk, then: Option<After>, window: &mut Window, cx: &mut Context<H>) {
    log::warn!("office: tab {id} is {risk:?}; its save becomes Save As");
    let queued = settle(host, id);
    if let Some(tab) = host.tab_mut(id) {
        tab.risk = Some(risk);
    }
    cx.notify();
    host.save_as(id, then, cx);
    run(host, queued, window, cx);
}

/// The save is over; the close and queue go on.
fn ended<H: Saving>(
    host: &mut H,
    id: u64,
    saved: Result<ely_files::Saved, SaveError>,
    landing: Landing,
    window: &mut Window,
    cx: &mut Context<H>,
) {
    let queued = settle(host, id);
    let Some(tab) = host.tab(id) else {
        // A write may land after its tab went.
        let said = match &saved {
            Ok(done) => Ok(done.path.as_path()),
            Err(_) => Err(format!("tab {id} closed during its save")),
        };
        host.core_mut().answer_saves(id, said);
        host.closed_first(id, landing.then, window, cx);
        return run(host, queued, window, cx);
    };
    // Edits after the export stay unsaved; a close asks again.
    let view = tab.view.read(cx);
    let changed = H::Engine::edited(view) != landing.edited || H::Engine::edit_pending(view);
    match landing.then {
        Some(after) if changed && saved.is_ok() => {
            host.finish(id, saved, landing.history, None, window, cx);
            host.request_close(id, after, window, cx);
        }
        then => host.finish(id, saved, landing.history, then, window, cx),
    }
    run(host, queued, window, cx);
}

/// Tab `id` exports no more; its waiting saves come back.
fn settle<H: Saving>(host: &mut H, id: u64) -> Vec<Waiting<H>> {
    if let Some(tab) = host.tab_mut(id) {
        tab.exporting = false;
    }
    let (queued, rest): (Vec<_>, Vec<_>) =
        host.core_mut().queued_saves.drain(..).partition(|(tab, _)| *tab == id);
    host.core_mut().queued_saves = rest;
    queued.into_iter().map(|(_, save)| save).collect()
}

fn run<H: Saving>(host: &mut H, queued: Vec<Waiting<H>>, window: &mut Window, cx: &mut Context<H>) {
    for save in queued {
        save(host, window, cx);
    }
}
