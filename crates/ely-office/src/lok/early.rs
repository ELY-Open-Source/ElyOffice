//! The first launch file, loaded while the window opens.

use std::{
    io,
    path::{Path, PathBuf},
    sync::Mutex,
};

use ely_lok::{Worker, sheet::csv::Import};

use super::{Loaded, Lok};
use crate::engine::{Engine, Load};

/// A launch file loaded by one worker.
struct Early {
    session: u64,
    path: PathBuf,
    loaded: io::Result<Loaded>,
}

/// The early load, until a load of its worker asks.
static EARLY: Mutex<Option<Early>> = Mutex::new(None);

/// Loads `path` and keeps it for the first load.
pub(super) fn load(worker: &mut Worker, path: PathBuf) {
    let loaded = Lok::load(worker, Load::File(&path, ""));
    if let Err(error) = &loaded {
        log::info!("lok: the early load failed: {error}");
    }
    let session = worker.session();
    match EARLY.lock() {
        Ok(mut slot) => {
            *slot = Some(Early {
                session,
                path,
                loaded,
            })
        }
        Err(_) => log::error!("lok: the early slot is poisoned"),
    }
}

/// The early load `from` asks for; else closed.
pub(super) fn taken(
    worker: &mut Worker,
    from: &Load<Import>,
) -> io::Result<Option<io::Result<Loaded>>> {
    let Some(early) = claim(worker, |_| true)? else {
        return Ok(None);
    };
    if asks(from, &early.path) {
        log::info!("lok: the launch file was loaded while the window opened");
        return Ok(Some(early.loaded));
    }
    // Another file comes first; the early one goes.
    log::info!("lok: the early load is not the one asked");
    close(worker, early.loaded).map(|()| None)
}

/// Closes the early load if a dropped `from` is it.
pub(super) fn skipped(worker: &mut Worker, from: &Load<Import>) -> io::Result<()> {
    match claim(worker, |early| asks(from, &early.path))? {
        Some(early) => {
            log::info!("lok: the launch file's tab let it go");
            close(worker, early.loaded)
        }
        None => Ok(()),
    }
}

/// Takes `worker`'s early load if `fits`; others stay.
fn claim(worker: &Worker, fits: impl FnOnce(&Early) -> bool) -> io::Result<Option<Early>> {
    let mut slot = EARLY
        .lock()
        .map_err(|_| io::Error::other("the early slot is poisoned"))?;
    match slot.take() {
        Some(early) if early.session == worker.session() && fits(&early) => Ok(Some(early)),
        other => {
            *slot = other;
            Ok(None)
        }
    }
}

/// Whether `from` asks for the early load of `path`.
fn asks(from: &Load<Import>, path: &Path) -> bool {
    matches!(from, Load::File(asked, "") if *asked == path)
}

fn close(worker: &mut Worker, early: io::Result<Loaded>) -> io::Result<()> {
    match early {
        Ok(loaded) => {
            worker.close_doc(loaded.doc)?;
            log::info!("lok: the early load closed");
            Ok(())
        }
        Err(error) => {
            log::info!("lok: the early load had failed: {error}");
            Ok(())
        }
    }
}

/// The early load's document, if one waits.
#[cfg(any(test, feature = "test-support"))]
pub(super) fn doc() -> Option<ely_lok::Doc> {
    let slot = EARLY.lock().expect("the early slot");
    let early = slot.as_ref()?;
    early.loaded.as_ref().ok().map(|loaded| loaded.doc.clone())
}

/// The author the shared settings name at launch.
pub(super) fn author() -> String {
    match ely_platform::settings::Shared::load() {
        Ok(shared) => shared.author,
        Err(error) => {
            log::error!("lok: settings unreadable, the login name signs: {error}");
            ely_platform::settings::Shared::default().author
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::Lok;
    use crate::engine::{Engine, Load};

    /// The slot is one per process.
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn fixture(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/office")
            .join(name)
    }

    #[test]
    fn the_first_load_takes_the_early_one_and_another_closes_it() {
        let _serial = SERIAL.lock().expect("the slot is ours");
        let Some(install) = ely_lok::test_install() else {
            return eprintln!("SKIP: no pinned LOK build");
        };
        let profile = tempfile::tempdir().expect("a profile dir");
        let mut worker = ely_lok::Worker::spawn(&install, profile.path()).expect("the worker");
        let (deck, text) = (fixture("t.pptx"), fixture("word.docx"));
        super::load(&mut worker, deck.clone());
        let early = super::doc().expect("the early load");
        let taken = Lok::load(&mut worker, Load::File(&deck, "")).expect("the deck");
        assert_eq!(taken.doc, early, "the same document, not a second load");
        worker.close_doc(taken.doc).expect("closes");
        super::load(&mut worker, deck.clone());
        let early = super::doc().expect("the early load");
        let other = Lok::load(&mut worker, Load::File(&text, "")).expect("the text");
        assert!(
            !other.doc.is_deck() && !other.doc.is_sheet(),
            "the asked file loads"
        );
        assert!(super::doc().is_none(), "the early one is spent");
        assert!(early.size(&mut worker).is_err(), "and closed");
        worker.close_doc(other.doc).expect("closes");
        worker.shutdown().expect("the worker ends");
    }

    #[test]
    fn a_skipped_launch_load_closes_and_others_leave_it() {
        let _serial = SERIAL.lock().expect("the slot is ours");
        let Some(install) = ely_lok::test_install() else {
            return eprintln!("SKIP: no pinned LOK build");
        };
        let profile = tempfile::tempdir().expect("a profile dir");
        let mut worker = ely_lok::Worker::spawn(&install, profile.path()).expect("the worker");
        let other = tempfile::tempdir().expect("a profile dir");
        let mut stranger = ely_lok::Worker::spawn(&install, other.path()).expect("a worker");
        let (deck, text) = (fixture("t.pptx"), fixture("word.docx"));
        super::load(&mut worker, deck.clone());
        let early = super::doc().expect("the early load");
        Lok::skipped(&mut worker, Load::File(&text, "")).expect("another file");
        Lok::skipped(&mut stranger, Load::File(&deck, "")).expect("another worker");
        let loaded = Lok::load(&mut stranger, Load::File(&text, "")).expect("its own load");
        assert_eq!(
            super::doc(),
            Some(early.clone()),
            "only its worker and file take it"
        );
        Lok::skipped(&mut worker, Load::File(&deck, "")).expect("its file");
        assert!(super::doc().is_none(), "the slot empties");
        assert!(early.size(&mut worker).is_err(), "the document closed");
        stranger.close_doc(loaded.doc).expect("closes");
        stranger.shutdown().expect("the stranger ends");
        worker.shutdown().expect("the worker ends");
    }
}
