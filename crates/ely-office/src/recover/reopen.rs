//! Points restored off the UI thread, then opened (§6.3).

use std::{
    io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use ely_files::{Recovery, RecoveryKey};
use gpui::{AppContext as _, AsyncApp, Context, WeakEntity};

use super::{Kept, Recovering, disk::remove};
use crate::{Opening as _, Source};

/// Restores each slot's newest point, then opens the copy.
pub(super) fn restore_off<H: Recovering>(
    host: &mut H,
    slots: Vec<(Kept, String)>,
    this_run: bool,
    then: impl FnOnce(&mut H, Vec<(String, io::Result<bool>)>, &mut Context<H>) + 'static,
    cx: &mut Context<H>,
) {
    let jobs: Vec<(u64, PathBuf, Kept, String)> = {
        let mut shared = host.core().shared.get_mut();
        let Some(keeping) = shared.keeping.as_mut() else { return };
        // Held meanwhile: a crash now still finds them.
        keeping.held.extend(slots.iter().map(|(kept, _)| kept.clone()));
        let folders: Vec<PathBuf> = slots.iter().map(|(kept, _)| keeping.folder(&kept.slot)).collect();
        slots
            .into_iter()
            .zip(folders)
            .map(|((kept, name), folder)| {
                // Each copy's folder is its future tab's.
                shared.next += 1;
                (shared.next - 1, folder, kept, name)
            })
            .collect()
    };
    host.flush_index(cx);
    let work = host.core().work();
    let task = cx.background_spawn(async move {
        jobs.into_iter()
            .map(|(id, folder, kept, name)| {
                let restored = restore(&kept, &folder, &work, id, this_run);
                // An empty slot has nothing to keep.
                let restored = match restored {
                    Ok(None) => remove(&folder).map(|()| None),
                    other => other,
                };
                (id, kept, name, restored)
            })
            .collect::<Vec<_>>()
    });
    cx.spawn(async move |this: WeakEntity<H>, cx: &mut AsyncApp| {
        let done = task.await;
        this.update(cx, |host, cx| {
            let shared = host.core().shared.clone();
            match shared.get_mut().keeping.as_mut() {
                Some(keeping) => keeping.held.retain(|held| !done.iter().any(|(_, kept, ..)| kept.slot == held.slot)),
                None => return,
            }
            let mut said = Vec::new();
            for (id, kept, name, restored) in done {
                let opened = match restored {
                    Ok(Some((copy, at))) => {
                        log::info!("recovery: reopening a {}", kept.format);
                        shared.get_mut().keeping.as_mut().expect("keeping").reopening.push((copy.clone(), at, kept));
                        host.load_into(id, Source::Copy(copy), cx);
                        Ok(true)
                    }
                    Ok(None) => Ok(false),
                    Err(error) => {
                        // Its points stay for a later start.
                        shared.get_mut().keeping.as_mut().expect("keeping").held.push(kept);
                        Err(error)
                    }
                };
                said.push((name, opened));
            }
            host.flush_index(cx);
            then(host, said, cx);
        })
        .ok();
    })
    .detach();
}

/// The newest point copied to the work folder, and when.
fn restore(kept: &Kept, folder: &Path, work: &Path, next: u64, this_run: bool) -> io::Result<Option<(PathBuf, SystemTime)>> {
    let recovery = match kept.sealed {
        true => {
            let key = RecoveryKey::load();
            // This run's points open with this run's key.
            if !key.lasting && !this_run {
                return Err(ely_ipc::Worded::error(io::ErrorKind::PermissionDenied, "error.no_key", &[]));
            }
            Recovery::sensitive(folder.to_path_buf(), key.key)
        }
        false => Recovery::new(folder.to_path_buf()),
    };
    let Some(point) = recovery.points()?.into_iter().next() else {
        return Ok(None);
    };
    let dir = crate::files::work_dir(work, next)?;
    let copy = dir.join(&kept.name).with_extension(&kept.format);
    recovery.restore(&point, &copy)?;
    Ok(Some((copy, point.at)))
}

