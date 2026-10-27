//! Another program reads, replaces and saves the slides' text.

use std::cell::RefCell;

use ely_office::prelude::*;
use gpui::{Entity, TestAppContext};

use super::{quiet, settle, update};
use crate::deck::Deck;

const DECK: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" xmlns:presentation="urn:oasis:names:tc:opendocument:xmlns:presentation:1.0" xmlns:officeooo="http://openoffice.org/2009/office" xmlns:dc="http://purl.org/dc/elements/1.1/" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
 <office:body><office:presentation>
  <draw:page draw:name="One"><draw:frame svg:x="2cm" svg:y="2cm" svg:width="20cm" svg:height="3cm"><draw:text-box><text:p>cat Cat 世界 😀 cat</text:p></draw:text-box></draw:frame><presentation:notes><draw:frame svg:x="2cm" svg:y="12cm" svg:width="16cm" svg:height="8cm" presentation:class="notes"><draw:text-box><text:p>cat in the notes</text:p></draw:text-box></draw:frame></presentation:notes></draw:page>
  <draw:page draw:name="Two"><draw:frame svg:x="2cm" svg:y="2cm" svg:width="8cm" svg:height="2cm"><table:table><table:table-column/><table:table-row><table:table-cell><text:p>cat cell</text:p></table:table-cell></table:table-row></table:table></draw:frame><draw:g><draw:frame svg:x="2cm" svg:y="8cm" svg:width="8cm" svg:height="2cm"><draw:text-box><text:p>grouped cat</text:p></draw:text-box></draw:frame></draw:g><officeooo:annotation svg:x="1cm" svg:y="1cm" svg:width="1cm" svg:height="1cm"><dc:creator>Ely</dc:creator><text:p>cat in a comment</text:p></officeooo:annotation></draw:page>
 </office:presentation></office:body>
</office:document>"#;

/// The answer to `words`, once it comes.
fn ask(cx: &mut TestAppContext, deck: &Entity<Deck>, words: &[&str]) -> Result<String, String> {
    let (ask, answered) = ely_ui::Ask::for_tests(&serde_json::json!(words).to_string());
    update(cx, deck, |deck, cx| deck.answer_request(ask, cx));
    let answer = RefCell::new(None);
    settle(cx, |_| {
        let mut got = answer.borrow_mut();
        if got.is_none() {
            *got = answered.try_recv().ok();
        }
        got.is_some()
    });
    answer.into_inner().expect("answered")
}

fn current(cx: &mut TestAppContext, deck: &Entity<Deck>) -> Option<usize> {
    cx.update(|app| deck.read(app).active_tab().map(|tab| tab.extra.slides.current))
}

#[test]
fn slide_text_reads_replaces_and_saves_leaving_notes_and_the_slide() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = super::deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("deck.fodp");
    std::fs::write(&path, DECK).expect("writes the deck");
    update(&mut cx, &deck, |deck, cx| deck.open_file(path.clone(), cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.slides.list.len() == 2));
    quiet(&cx, &deck);
    let file = path.display().to_string();
    let listed = ask(&mut cx, &deck, &["docs"]).expect("docs");
    assert_eq!(std::fs::canonicalize(&listed).ok(), std::fs::canonicalize(&path).ok(), "{listed}");
    assert_eq!(ask(&mut cx, &deck, &["get", &file]), Ok("cat Cat 世界 😀 cat\x0ccat cell\ngrouped cat".into()));
    update(&mut cx, &deck, |deck, cx| deck.go_to_finding(1, cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.slides.current == 1));
    assert_eq!(ask(&mut cx, &deck, &["replace", &file, "cat", "dog"]), Ok(String::new()));
    let after = "dog Cat 世界 😀 dog\x0cdog cell\ngrouped dog";
    assert_eq!(ask(&mut cx, &deck, &["get", &file]), Ok(after.into()));
    quiet(&cx, &deck);
    assert_eq!(current(&mut cx, &deck), Some(1), "the user's slide stays");
    cx.update(|app| {
        let tab = deck.read(app).active_tab().expect("a tab");
        assert!(tab.dirty(app), "a read leaves the edit unsaved");
        assert_eq!(tab.session.path().map(std::path::PathBuf::from), Some(path.clone()), "a read keeps the file");
    });
    // Impress: each changed text box is one undo step.
    update(&mut cx, &deck, |deck, cx| deck.run_active("edit.undo", &[], cx));
    quiet(&cx, &deck);
    assert_eq!(ask(&mut cx, &deck, &["get", &file]), Ok(after.replace("grouped dog", "grouped cat")));
    update(&mut cx, &deck, |deck, cx| deck.run_active("edit.redo", &[], cx));
    quiet(&cx, &deck);
    assert_eq!(ask(&mut cx, &deck, &["get", &file]), Ok(after.into()));
    let missing = ask(&mut cx, &deck, &["replace", &file, "zebra", "x"]);
    assert!(missing.as_ref().is_err_and(|why| why.starts_with("nothing replaced")), "{missing:?}");
    assert!(ask(&mut cx, &deck, &["replace", &file, "dog", "a\nb"]).is_err(), "a break in WITH");
    let other = dir.path().join("other.odp");
    std::fs::write(&other, b"x").expect("a stray file");
    assert!(ask(&mut cx, &deck, &["get", &other.display().to_string()]).is_err(), "a file not open");
    let saved = ask(&mut cx, &deck, &["save", &file]).expect("saved");
    assert_eq!(std::fs::canonicalize(&saved).ok(), std::fs::canonicalize(&path).ok());
    let on_disk = std::fs::read_to_string(&path).expect("reads the saved deck");
    assert_eq!(ely_office::remote::slide_text(on_disk.as_bytes()).ok().as_deref(), Some(after), "on disk at the answer");
    assert!(on_disk.contains("cat in the notes"), "notes untouched");
    assert!(on_disk.contains("cat in a comment"), "comments untouched");
    assert_eq!(ask(&mut cx, &deck, &["replace", &file, " ", "\t"]), Ok(String::new()));
    assert_eq!(ask(&mut cx, &deck, &["get", &file]), Ok(after.replace(' ', "\t")));
}
