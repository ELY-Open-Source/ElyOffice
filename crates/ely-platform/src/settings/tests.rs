use serde_json::json;

use super::{Store, VERSION};

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::at(dir.path().join("ElyOffice").join("settings.json"));
    (dir, store)
}

#[test]
fn an_absent_file_is_empty_and_a_write_makes_it_versioned() {
    let (_dir, store) = store();
    assert!(store.read().unwrap().is_empty());
    store.set("appearance", json!("dark")).unwrap();
    let read = store.read().unwrap();
    assert_eq!(read["appearance"], "dark");
    assert_eq!(read["version"], VERSION);
}

#[test]
fn a_write_keeps_fields_another_app_wrote() {
    let (_dir, store) = store();
    std::fs::create_dir_all(store.path().parent().unwrap()).unwrap();
    std::fs::write(store.path(), r#"{"version":1,"later":{"kept":true},"language":"en"}"#).unwrap();
    store.set("language", json!("zh-CN")).unwrap();
    let read = store.read().unwrap();
    assert_eq!(read["language"], "zh-CN");
    assert_eq!(read["later"], json!({"kept": true}), "unknown fields stay");
}

#[test]
fn two_writers_at_once_lose_nothing() {
    let (_dir, store) = store();
    let writers: Vec<_> = (0..8)
        .map(|n| {
            let store = store.clone();
            std::thread::spawn(move || store.set(&format!("field{n}"), json!(n)).unwrap())
        })
        .collect();
    for writer in writers {
        writer.join().unwrap();
    }
    let read = store.read().unwrap();
    assert!((0..8).all(|n| read[&format!("field{n}")] == n), "{read:?}");
}

#[test]
fn a_newer_or_broken_file_is_refused_and_left_alone() {
    let (_dir, store) = store();
    std::fs::create_dir_all(store.path().parent().unwrap()).unwrap();
    for text in [r#"{"version":2}"#, r#"{"version":"1"}"#, "[1]", "{"] {
        std::fs::write(store.path(), text).unwrap();
        assert!(store.read().is_err(), "{text} is refused");
        assert!(store.set("language", json!("en")).is_err(), "{text} is not overwritten");
        assert_eq!(std::fs::read_to_string(store.path()).unwrap(), text);
    }
}

#[test]
fn shared_fields_type_strictly_and_default_when_absent() {
    use super::{Appearance, Language, Motion, Shared};
    let fields = |text: &str| serde_json::from_str::<serde_json::Map<_, _>>(text).unwrap();
    assert_eq!(Shared::from_fields(&fields("{}")).unwrap(), Shared::default());
    let read = Shared::from_fields(&fields(
        r#"{"appearance":"dark","language":"zh-CN","motion":"reduce","omarchy_sync":false,"author":" Ana ","check_updates":true}"#,
    ))
    .unwrap();
    assert!(read.check_updates && !Shared::default().check_updates, "asked for, never by default");
    assert_eq!(
        (read.appearance, read.language, read.motion, read.omarchy_sync, read.author.as_str()),
        (Appearance::Dark, Language::Chinese, Motion::Reduced, false, "Ana")
    );
    for bad in [r#"{"appearance":"blue"}"#, r#"{"omarchy_sync":"yes"}"#, r#"{"author":3}"#, r#"{"check_updates":1}"#] {
        assert!(Shared::from_fields(&fields(bad)).is_err(), "{bad}");
    }
}

#[test]
fn interface_text_takes_a_family_and_a_size_within_bounds() {
    use super::{Shared, UI_TEXT_SIZES};
    let fields = |text: &str| serde_json::from_str::<serde_json::Map<_, _>>(text).unwrap();
    let default = Shared::default();
    assert_eq!((default.ui_font.as_str(), default.ui_text_size), ("", UI_TEXT_SIZES.2));
    let read = Shared::from_fields(&fields(r#"{"ui_font":" Noto Sans CJK SC ","ui_text_size":16}"#)).unwrap();
    assert_eq!((read.ui_font.as_str(), read.ui_text_size), ("Noto Sans CJK SC", 16));
    assert_eq!(Shared::from_fields(&fields(r#"{"ui_text_size":12}"#)).unwrap().ui_text_size, 12);
    for bad in [r#"{"ui_font":1}"#, r#"{"ui_text_size":11}"#, r#"{"ui_text_size":17}"#, r#"{"ui_text_size":13.5}"#, r#"{"ui_text_size":"13"}"#] {
        assert!(Shared::from_fields(&fields(bad)).is_err(), "{bad}");
    }
}

#[test]
fn the_document_zoom_is_automatic_or_a_listed_percent() {
    use super::{DOC_ZOOMS, DocZoom, Shared};
    let fields = |text: &str| serde_json::from_str::<serde_json::Map<_, _>>(text).unwrap();
    assert_eq!(Shared::default().doc_zoom, DocZoom::Auto);
    assert_eq!(Shared::from_fields(&fields(r#"{"doc_zoom":"auto"}"#)).unwrap().doc_zoom, DocZoom::Auto);
    for percent in DOC_ZOOMS {
        let read = Shared::from_fields(&fields(&format!(r#"{{"doc_zoom":{percent}}}"#))).unwrap();
        assert_eq!(read.doc_zoom, DocZoom::Percent(percent));
        assert_eq!(DocZoom::Percent(percent).value(), json!(percent));
    }
    assert_eq!(DocZoom::Auto.value(), json!("auto"));
    for bad in [r#"{"doc_zoom":110}"#, r#"{"doc_zoom":"fit"}"#, r#"{"doc_zoom":true}"#, r#"{"doc_zoom":1.5}"#] {
        assert!(Shared::from_fields(&fields(bad)).is_err(), "{bad}");
    }
}

#[test]
fn two_sided_printing_is_the_printers_unless_chosen() {
    use super::{Shared, Sides, word};
    let fields = |text: &str| serde_json::from_str::<serde_json::Map<_, _>>(text).unwrap();
    assert_eq!(Shared::default().two_sided, Sides::Printer);
    for (sides, stored) in Sides::ALL {
        let read = Shared::from_fields(&fields(&format!(r#"{{"two_sided":"{stored}"}}"#))).unwrap();
        assert_eq!(read.two_sided, sides);
        assert_eq!(word(&Sides::ALL, &sides), stored);
    }
    assert_eq!(Sides::ALL.map(|(_, stored)| stored), ["printer", "one", "long", "short"]);
    for bad in [r#"{"two_sided":"both"}"#, r#"{"two_sided":true}"#, r#"{"two_sided":2}"#] {
        assert!(Shared::from_fields(&fields(bad)).is_err(), "{bad}");
    }
}

#[test]
fn a_write_overtaken_while_waiting_leaves_the_file() {
    let (_dir, store) = store();
    store.set("author", json!("Ana")).unwrap();
    let held = crate::lock::FileLock::take(&store.path().parent().unwrap().join(super::LOCK)).unwrap();
    let waiting = {
        let store = store.clone();
        std::thread::spawn(move || store.set_if("author", json!("Bob"), || false).unwrap())
    };
    std::thread::sleep(std::time::Duration::from_millis(200));
    drop(held);
    assert!(!waiting.join().unwrap(), "the overtaken write is skipped");
    assert_eq!(store.read().unwrap()["author"], "Ana");
}

#[test]
fn spanish_language_is_read_and_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let store = super::Store::at(dir.path().join("shared.json"));
    store.set("language", json!("es-ES")).unwrap();
    store.set("author", json!("Ana")).unwrap();
    let fields = store.read().unwrap();
    assert_eq!(super::Shared::from_fields(&fields).unwrap().language, super::Language::Spanish);
    assert_eq!(fields["language"], "es-ES");
}
