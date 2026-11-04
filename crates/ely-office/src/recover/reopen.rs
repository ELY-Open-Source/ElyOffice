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
    let Some(keeping) = host.core_mut().keeping.as_mut() else { return };
    // Held meanwhile: a crash now still finds them.
    keeping.held.extend(slots.iter().map(|(kept, _)| kept.clone()));
    let jobs: Vec<(u64, PathBuf, Kept, String)> = slots
        .into_iter()
        .map(|(kept, name)| {
            let folder = host.core().keeping.as_ref().expect("keeping").folder(&kept.slot);
            let id = host.core().next;
            // Each copy's folder is its future tab's.
            host.core_mut().next += 1;
            (id, folder, kept, name)
        })
        .collect();
    host.flush_index(cx);
    let work = host.core().work.clone();
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
            let Some(keeping) = host.core_mut().keeping.as_mut() else { return };
            keeping.held.retain(|held| !done.iter().any(|(_, kept, ..)| kept.slot == held.slot));
            let mut said = Vec::new();
            for (id, kept, name, restored) in done {
                let keeping = host.core_mut().keeping.as_mut().expect("keeping");
                let opened = match restored {
                    Ok(Some((copy, at))) => {
                        log::info!("recovery: reopening a {}", kept.format);
                        keeping.reopening.push((copy.clone(), at, kept));
                        host.load_into(id, Source::Copy(copy), cx);
                        Ok(true)
                    }
                    Ok(None) => Ok(false),
                    Err(error) => {
                        // Its points stay for a later start.
                        keeping.held.push(kept);
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

