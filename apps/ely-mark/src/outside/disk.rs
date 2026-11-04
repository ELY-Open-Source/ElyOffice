//! A tab's file read or sought off the window.

use std::{
    io,
    path::{Path, PathBuf},
};

use ely_files::Fingerprint;
use ely_office::prelude::*;
use gpui::{AppContext as _, Context};

use super::{GAP, Outside};
use crate::desk::Desk;
use crate::text::Source;

/// Where a missing file went.
pub(super) enum Gone {
    Back,
    Renamed(PathBuf, ely_files::Key),
    Deleted,
}

/// A missing `path`, once file `had`: back, renamed, or deleted.
fn gone_to(path: &Path, had: Option<(u64, u64)>) -> io::Result<Gone> {
    if path.try_exists()? {
        return Ok(Gone::Back);
    }
    let renamed = match (had, path.parent()) {
        (Some(had), Some(dir)) => ely_files::Sessions::renamed_in(had, dir)?,
        _ => None,
    };
    match renamed {
        Some(to) => {
            let key = ely_files::Sessions::key(&to)?;
            Ok(Gone::Renamed(to, key))
        }
        None => Ok(Gone::Deleted),
    }
}

/// The file's bytes and the fingerprint they match.
fn read_still(path: &Path) -> io::Result<(Fingerprint, Source)> {
    // A writer mid-way changes the fingerprint; read again.
    for _ in 0..5 {
        let before = Fingerprint::of(path)?;
        let bytes = std::fs::read(path)?;
        if Fingerprint::of(path)? == before {
            return Ok((before, Source::decode(bytes)?));
        }
    }
    Err(ely_ipc::Worded::error(io::ErrorKind::Other, "mark.error.unsettled", &[]))
}

impl Desk {
    /// Gone: an atomic replace, a rename, or a delete.
    pub(super) fn missing(&mut self, id: u64, path: PathBuf, ours: Option<Fingerprint>, cx: &mut Context<Self>) {
        let Some(tab) = self.tab(id) else { return };
        let began = (ours, tab.extra.stamp);
        let had = tab.registry.and_then(|registry| self.core.sessions.identity_of(registry));
        cx.spawn(async move |desk, cx| {
            cx.background_executor().timer(GAP).await;
            let looking = path.clone();
            let found = cx.background_spawn(async move { gone_to(&looking, had) }).await;
            desk.update(cx, |desk, cx| desk.still_missing(id, path, began, found, cx))
                .ok();
        })
        .detach();
    }

    pub(super) fn still_missing(
        &mut self,
        id: u64,
        path: PathBuf,
        (ours, stamp): (Option<Fingerprint>, u64),
        found: io::Result<Gone>,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.checked(id, &path, stamp) else {
            return log::info!("mark: a search for tab {id}'s file lapsed");
        };
        // A save landed meanwhile: look again.
        if tab.session.fingerprint() != ours {
            return self.look(id, cx);
        }
        match found {
            Ok(Gone::Back) => self.look(id, cx),
            Ok(Gone::Renamed(to, key)) => {
                log::info!("mark: {} was renamed outside", path.display());
                // A refused follow stays deleted; no look loops.
                if self.follow(id, to, key, cx) {
                    self.look(id, cx);
                }
            }
            Ok(Gone::Deleted) => {
                log::warn!("mark: {} was deleted outside", path.display());
                self.set_outside(id, Outside::Deleted, cx);
            }
            Err(error) => self.set_outside(id, Outside::Unread(ely_ui::i18n::error(cx, &error).to_string()), cx),
        }
    }

    /// Changed: a clean tab follows, a dirty one asks.
    pub(super) fn read_disk(&mut self, id: u64, path: PathBuf, cx: &mut Context<Self>) {
        let Some(tab) = self.tab_mut(id) else {
            return;
        };
        tab.extra.reads += 1;
        let number = (tab.extra.reads, tab.extra.stamp);
        let reading = path.clone();
        cx.spawn(async move |desk, cx| {
            let read = cx.background_spawn(async move { read_still(&reading) }).await;
            desk.update(cx, |desk, cx| desk.disk_read(id, path, number, read, cx))
                .ok();
        })
        .detach();
    }

    /// A read counts once the disk is seen again.
    pub(super) fn disk_read(
        &mut self,
        id: u64,
        path: PathBuf,
        number: (u64, u64),
        read: io::Result<(Fingerprint, Source)>,
        cx: &mut Context<Self>,
    ) {
        if !self.reading(id, &path, number) {
            return;
        }
        let read = match read {
            Ok(read) => read,
            Err(error) => return self.set_outside(id, Outside::Unread(ely_ui::i18n::error(cx, &error).to_string()), cx),
        };
        let seeing = path.clone();
        cx.spawn(async move |desk, cx| {
            let now = cx.background_spawn(async move { Fingerprint::of(&seeing).ok() }).await;
            desk.update(cx, |desk, cx| {
                if !desk.reading(id, &path, number) {
                    return;
                }
                // The disk moved on since the read: look again.
                if now != Some(read.0) {
                    log::info!("mark: tab {id}'s file changed while read");
                    return desk.look(id, cx);
                }
                desk.take_read(id, &path, read, cx);
            })
            .ok();
        })
        .detach();
    }

    /// Whether read `number` is tab `id`'s newest and current.
    fn reading(&self, id: u64, path: &Path, (number, stamp): (u64, u64)) -> bool {
        let Some(tab) = self.checked(id, path, stamp) else {
            log::info!("mark: a read for tab {id} lapsed");
            return false;
        };
        let newest = tab.extra.reads == number;
        if !newest {
            log::info!("mark: read {number} of tab {id} lapsed");
        }
        newest
    }

    /// The disk's text meets the tab's: follow, keep, or ask.
    fn take_read(&mut self, id: u64, path: &Path, (fingerprint, disk): (Fingerprint, Source), cx: &mut Context<Self>) {
        let Some(tab) = self.tab(id) else { return };
        let current = tab.doc.lock().expect("the text lock").current.clone();
        let kept = disk.text == tab.extra.base;
        let dirty = tab.dirty(cx);
        log::info!("mark: {} changed outside; unsaved {dirty}", path.display());
        match (disk.text == current, dirty) {
            // The same text: only the file underneath is new.
            (true, _) => {
                self.adopt_disk(id, &disk, fingerprint, cx);
            }
            // The text as last saved: the edits stay unsaved.
            (false, true) if kept => self.keep_edits(id, disk, fingerprint, cx),
            (false, false) => self.take_disk(id, disk, fingerprint, cx),
            (false, true) => {
                let Some(tab) = self.tab_mut(id) else { return };
                tab.session.external_modified();
                tab.extra.policy.external_modified();
                self.set_outside(id, Outside::Changed(disk, fingerprint), cx);
                // An open comparison shows the newest disk text.
                if self
                    .comparing
                    .as_ref()
                    .is_some_and(|comparing| comparing.id == id)
                {
                    self.compare(id, cx);
                }
            }
        }
    }
}
