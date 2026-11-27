//! Multilevel schemes on the real kernel, through Word's format (#130).

use std::io::Read as _;

use ely_office::panel::Panel as _;
use ely_office::prelude::*;
use gpui::{Entity, TestAppContext};

use super::Props;
use super::levels::{LEVELS, SCHEMES, levels, scheme_of};
use crate::docs::Docs;
use crate::tests::{docs, settle, settled, with_window, written};

/// "One", then "Two" a level below, unstyled.
const NESTED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:list><text:list-item><text:p>One</text:p>
 <text:list><text:list-item><text:p>Two</text:p></text:list-item></text:list>
 </text:list-item></text:list></office:text></office:body>
</office:document>"#;

/// Word keeps nine levels; Writer has ten.
fn nine(state: &str) -> Vec<&str> {
    state.split(';').take(9).collect()
}

fn opened(cx: &mut TestAppContext, docs: &Entity<Docs>, path: std::path::PathBuf) -> Entity<Props> {
    let before = cx.update(|app| docs.read(app).core.tabs.len());
    with_window(cx, docs, |docs, _, cx| docs.open_file(path, cx));
    settle(cx, |app| docs.read(app).core.tabs.len() > before);
    settled(cx, docs);
    super::tests::panel(cx, docs)
}

/// Word's numbering part of a saved file.
fn numbering(path: &std::path::Path) -> String {
    let mut file = zip::ZipArchive::new(std::fs::File::open(path).expect("the file opens")).expect("the file unzips");
    let mut xml = String::new();
    file.by_name("word/numbering.xml").expect("a numbering part").read_to_string(&mut xml).expect("UTF-8");
    xml
}

#[test]
fn each_scheme_numbers_the_levels_and_word_keeps_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    cx.update(ely_gpui_component::init_for_tests);
    let source = dir.path().join("nested.fodt");
    std::fs::write(&source, NESTED).expect("writes");
    let props = opened(&mut cx, &docs, source);
    let state = |app: &gpui::App, props: &Entity<Props>| props.read(app).state(LEVELS, app).map(|it| it.to_string());
    let run = |cx: &mut TestAppContext, id: &str, values: &[&str]| {
        cx.update(|app| props.update(app, |props, cx| props.run(id, values, cx)));
    };
    settle(&cx, |app| state(app, &props).is_some_and(|it| !it.is_empty()));
    let unstyled = cx.update(|app| state(app, &props)).expect("a list state");
    assert_eq!(scheme_of(&unstyled), None, "the file's own list shows as custom");
    run(&mut cx, "edit.select_all", &[]);
    let id = cx.update(|app| docs.read(app).core.active().expect("a tab"));
    let mut saved = Vec::new();
    for (key, cycle) in SCHEMES {
        let wanted = levels(cycle);
        run(&mut cx, "list.levels", &[&wanted]);
        settle(&cx, |app| state(app, &props).as_deref() == Some(wanted.as_str()));
        assert_eq!(scheme_of(&wanted), Some(key), "the picker shows it");
        let path = dir.path().join(format!("{}.docx", key.rsplit('.').next().expect("a name")));
        with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, path.clone(), None, window, cx));
        written(&cx, &path, None);
        // Word shows level two as the scheme does.
        let two = if cycle.is_empty() { "%1.%2.".to_string() } else { cycle[1].1.replace('#', "%2") };
        let xml = numbering(&path);
        assert!(xml.contains(&format!(r#"<w:lvlText w:val="{two}"/>"#)), "{key}: level two in Word: {xml}");
        saved.push((key, wanted, path));
    }
    // One step each: five back to the file's list.
    for _ in SCHEMES {
        run(&mut cx, "edit.undo", &[]);
    }
    settle(&cx, |app| state(app, &props).as_deref() == Some(unstyled.as_str()));
    // Reopened, Word's files give the same nine levels back.
    for (key, wanted, path) in saved {
        // A copy: the tab holds its last saved file.
        let copy = path.with_extension("back.docx");
        std::fs::copy(&path, &copy).expect("copies");
        let back = opened(&mut cx, &docs, copy);
        settle(&cx, |app| state(app, &back).is_some_and(|it| !it.is_empty()));
        let reread = cx.update(|app| state(app, &back)).expect("a list state");
        assert_eq!(nine(&reread), nine(&wanted), "{key}: Word's file keeps the scheme");
    }
}

#[test]
fn schemes_spell_ten_levels_and_repeat() {
    assert_eq!(levels(SCHEMES[0].1).split(';').nth(2), Some("4|%1%.%2%.%3%."));
    assert_eq!(levels(SCHEMES[2].1).split(';').nth(5), Some("15|%6%、"), "the cycle starts again at six");
    assert_eq!(levels(SCHEMES[1].1).split(';').count(), 10);
}
