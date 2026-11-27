//! An app's on-off choices, each kept in its settings file.

use std::{
    collections::HashMap,
    io,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use ely_platform::{AppId, settings::Store};
use gpui::{App, AsyncApp, Global};
use serde_json::Value;

/// One choice: its key, and what absent means.
#[derive(Clone, Copy)]
pub struct Choice {
    pub key: &'static str,
    default: bool,
}

/// Ordinary files save on their own (§6.2).
pub const AUTOSAVE: Choice = Choice { key: "autosave", default: false };
/// New mail shows a system notification (MAIL-15).
pub const NOTIFY: Choice = Choice { key: "notifications", default: true };
/// Tracked deletions stay struck in the text (#143).
pub const STRUCK: Choice = Choice { key: "deletions_struck", default: false };
const ALL: [Choice; 3] = [AUTOSAVE, NOTIFY, STRUCK];

/// Each choice and its latest switch.
struct Switches {
    store: Option<Store>,
    on: HashMap<&'static str, (bool, Arc<AtomicU64>)>,
}

impl Global for Switches {}

/// Reads `app`'s choices; a bad one is reported, then defaulted.
pub(super) fn install(app: AppId, cx: &mut App) {
    let store = Store::app(app);
    let saved = store
        .as_ref()
        .map_err(|error| io::Error::new(error.kind(), error.to_string()))
        .and_then(|store| store.read());
    let mut on = HashMap::new();
    for choice in ALL {
        let read = saved.as_ref().map_err(|error| io::Error::new(error.kind(), error.to_string())).and_then(|fields| parse(choice, fields.get(choice.key)));
        let value = read.unwrap_or_else(|error| {
            log::error!("settings: {} unreadable: {error}", choice.key);
            cx.global_mut::<super::Current>().app_error.insert(choice.key, error.to_string().into());
            choice.default
        });
        on.insert(choice.key, (value, Arc::default()));
    }
    cx.set_global(Switches { store: store.ok(), on });
}

/// Absent is the default; else a boolean.
fn parse(choice: Choice, saved: Option<&Value>) -> io::Result<bool> {
    match saved {
        None => Ok(choice.default),
        Some(Value::Bool(on)) => Ok(*on),
        Some(other) => Err(io::Error::new(io::ErrorKind::InvalidData, format!("settings: {} cannot be {other}", choice.key))),
    }
}

/// Defaults without reading the file: for tests.
#[cfg(any(test, feature = "test-support"))]
pub(super) fn install_defaults(cx: &mut App) {
    let on = ALL.iter().map(|choice| (choice.key, (choice.default, Arc::default()))).collect();
    cx.set_global(Switches { store: None, on });
}

/// Whether `choice` is on.
pub fn on(choice: Choice, cx: &App) -> bool {
    cx.global::<Switches>().on[choice.key].0
}

/// Takes the switch now; the latest one is written.
pub fn set(choice: Choice, on: bool, cx: &mut App) {
    let switches = cx.global_mut::<Switches>();
    let entry = switches.on.get_mut(choice.key).expect("every choice is read");
    entry.0 = on;
    let latest = entry.1.clone();
    let turn = latest.fetch_add(1, Ordering::SeqCst) + 1;
    let Some(store) = switches.store.clone() else {
        return super::unsaved(choice.key, Some("no settings file for this app".into()), cx);
    };
    cx.spawn(async move |cx: &mut AsyncApp| {
        let written = cx.background_executor().spawn(async move { save(&store, choice, on, turn, &latest) }).await;
        cx.update(|cx| {
            super::unsaved(choice.key, written.err().map(|error| {
                log::error!("settings: {} not saved: {error}", choice.key);
                error.to_string().into()
            }), cx)
        });
    })
    .detach();
}

/// Writes switch `turn` unless a later one came first.
fn save(store: &Store, choice: Choice, on: bool, turn: u64, latest: &AtomicU64) -> io::Result<bool> {
    store.set_if(choice.key, Value::Bool(on), || latest.load(Ordering::SeqCst) == turn)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use std::sync::atomic::AtomicU64;

    use ely_platform::settings::Store;

    use super::{AUTOSAVE, NOTIFY, parse, save};

    #[test]
    fn a_switch_reads_strictly_and_absent_is_its_default() {
        assert!(!parse(AUTOSAVE, None).unwrap());
        assert!(parse(NOTIFY, None).unwrap());
        assert!(parse(AUTOSAVE, Some(&json!(true))).unwrap());
        assert!(!parse(NOTIFY, Some(&json!(false))).unwrap());
        for bad in [json!("yes"), json!(1), json!(null)] {
            assert!(parse(AUTOSAVE, Some(&bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn an_earlier_switch_saved_late_leaves_the_latest() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path().join("settings.json"));
        let latest = AtomicU64::new(2);
        // On, then off: the later save runs first.
        assert!(save(&store, NOTIFY, false, 2, &latest).unwrap());
        assert!(!save(&store, NOTIFY, true, 1, &latest).unwrap(), "the earlier switch is dropped");
        assert_eq!(store.read().unwrap()["notifications"], json!(false));
    }
}
