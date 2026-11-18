//! The first launch file, loaded while the window opens.

use std::{io, path::PathBuf, sync::Mutex};

use ely_lok::{Worker, sheet::csv::Import};

use super::{Loaded, Lok};
use crate::engine::{Engine, Load};

/// The early load and its file, until a load asks.
static EARLY: Mutex<Option<(PathBuf, io::Result<Loaded>)>> = Mutex::new(None);

/// Loads `path` and keeps it for the first load.
pub(super) fn load(worker: &mut Worker, path: PathBuf) {
    let loaded = Lok::load(worker, Load::File(&path, ""));
    if let Err(error) = &loaded {
        log::info!("lok: the early load failed: {error}");
    }
    match EARLY.lock() {
        Ok(mut slot) => *slot = Some((path, loaded)),
        Err(_) => log::error!("lok: the early slot is poisoned"),
    }
}

/// The early load `from` asks for; else closed.
pub(super) fn taken(
    worker: &mut Worker,
    from: &Load<Import>,
) -> io::Result<Option<io::Result<Loaded>>> {
    let slot = EARLY
        .lock()
        .map_err(|_| io::Error::other("the early slot is poisoned"))?
        .take();
    let Some((path, early)) = slot else {
        return Ok(None);
    };
    match from {
        Load::File(asked, "") if *asked == path => {
            log::info!("lok: the launch file was loaded while the window opened");
            return Ok(Some(early));
        }
        // Another file comes first; the early one goes.
        _ => match early {
            Ok(loaded) => {
                log::info!("lok: the early load is not the one asked; closed");
                worker.close_doc(loaded.doc)?;
            }
            Err(error) => log::info!("lok: the early load failed and is not asked: {error}"),
        },
    }
    Ok(None)
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

    fn fixture(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/office")
            .join(name)
    }

    #[test]
    fn the_first_load_takes_the_early_one_and_another_closes_it() {
        let Some(install) = ely_lok::test_install() else {
            return eprintln!("SKIP: no pinned LOK build");
        };
        let profile = tempfile::tempdir().expect("a profile dir");
        let mut worker = ely_lok::Worker::spawn(&install, profile.path()).expect("the worker");
        let (deck, text) = (fixture("t.pptx"), fixture("word.docx"));
        super::load(&mut worker, deck.clone());
        let early = super::EARLY
            .lock()
            .expect("the slot")
            .as_ref()
            .map(|(_, loaded)| loaded.as_ref().expect("loads").doc.clone());
        let taken = Lok::load(&mut worker, Load::File(&deck, "")).expect("the deck");
        assert_eq!(
            Some(taken.doc.clone()),
            early,
            "the same document, not a second load"
        );
        worker.close_doc(taken.doc).expect("closes");
        super::load(&mut worker, deck);
        let other = Lok::load(&mut worker, Load::File(&text, "")).expect("the text");
        assert!(
            !other.doc.is_deck() && !other.doc.is_sheet(),
            "the asked file loads"
        );
        assert!(
            super::EARLY.lock().expect("the slot").is_none(),
            "the early one is spent"
        );
        worker.close_doc(other.doc).expect("closes");
        worker.shutdown().expect("the worker ends");
    }
}
