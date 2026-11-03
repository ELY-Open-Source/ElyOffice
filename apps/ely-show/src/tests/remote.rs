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

/// Indices of the selected objects on the shown slide.
fn marked(app: &gpui::App, deck: &Entity<Deck>) -> Vec<usize> {
    let objects = deck.read(app).active_tab().and_then(|tab| tab.extra.objects.as_ref());
    objects.map_or(Vec::new(), |found| found.objects.iter().enumerate().filter(|(_, o)| o.selected).map(|(at, _)| at).collect())
}

fn current(cx: &mut TestAppContext, deck: &Entity<Deck>) -> Option<usize> {
    cx.update(|app| deck.read(app).active_tab().map(|tab| tab.extra.slides.current))
}

/// Reads done and the revision read, of tab `id`.
fn reads(app: &gpui::App, deck: &Entity<Deck>, id: u64) -> (u64, Option<ely_lok::Doc>) {
    let tab = deck.read(app).tab(id).expect("the tab");
    (tab.extra.reads, tab.extra.read_at.clone())
}

#[test]
fn a_hidden_deck_edited_from_outside_reads_nothing_until_shown() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = super::deck(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let path = dir.path().join("deck.fodp");
    std::fs::write(&path, DECK).expect("writes the deck");
    update(&mut cx, &deck, |deck, cx| deck.open_file(path.clone(), cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.slides.list.len() == 2));
    quiet(&cx, &deck);
    let hidden = cx.update(|app| deck.read(app).core.active()).expect("a tab");
    update(&mut cx, &deck, |deck, cx| deck.new_blank(cx));
    settle(&cx, |app| deck.read(app).core.active() != Some(hidden));
    quiet(&cx, &deck);
    let before = cx.update(|app| reads(app, &deck, hidden));
    let view = |app: &gpui::App| deck.read(app).tab(hidden).expect("the tab").view.read(app).read_at();
    let seen = cx.update(|app| view(app));
    let file = path.display().to_string();
    for (find, with) in [("cat", "dog"), ("dog", "cow")] {
        assert_eq!(ask(&mut cx, &deck, &["replace", &file, find, with]), Ok(String::new()));
    }
    // The hidden view takes the kernel's report first.
    settle(&cx, |app| view(app).newer_than(&seen));
    cx.run_until_parked();
    // A hundred hidden decks rereading would hold the engine.
    assert_eq!(cx.update(|app| reads(app, &deck, hidden)), before, "a hidden deck reads nothing");
    assert!(ask(&mut cx, &deck, &["save", &file]).is_ok(), "saved");
    let on_disk = std::fs::read_to_string(&path).expect("reads the saved deck");
    let after = "cow Cat 世界 😀 cow\x0ccow cell\ngrouped cow";
    assert_eq!(ely_office::remote::slide_text(on_disk.as_bytes()).ok().as_deref(), Some(after), "both edits saved");
    update(&mut cx, &deck, |deck, cx| {
        deck.core.set_active(Some(hidden));
        cx.notify();
    });
    // Shown, it reads the state the edits left.
    settle(&cx, |app| {
        let (done, at) = reads(app, &deck, hidden);
        let seen = deck.read(app).tab(hidden).expect("the tab").view.read(app).read_at();
        done > before.0 && at.is_some_and(|at| !seen.newer_than(&at))
    });
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
    // A selected table: matches elsewhere still change.
    update(&mut cx, &deck, |deck, cx| deck.select_objects(&["0".into()], cx));
    settle(&cx, |app| marked(app, &deck) == vec![0]);
    assert_eq!(ask(&mut cx, &deck, &["replace", &file, "cat", "dog"]), Ok(String::new()));
    let after = "dog Cat 世界 😀 dog\x0cdog cell\ngrouped dog";
    assert_eq!(ask(&mut cx, &deck, &["get", &file]), Ok(after.into()));
    quiet(&cx, &deck);
    assert_eq!(current(&mut cx, &deck), Some(1), "the user's slide stays");
    update(&mut cx, &deck, |deck, cx| deck.force_reread(deck.core.active().expect("a tab"), cx));
    quiet(&cx, &deck);
    assert_eq!(cx.update(|app| marked(app, &deck)), vec![0], "the selection stays");
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
    // A shape picked inside the entered group (F3): refused, unchanged.
    let id = cx.update(|app| deck.read(app).core.active()).expect("a tab");
    // Picks by number need a read after the replaces.
    update(&mut cx, &deck, |deck, cx| deck.force_reread(id, cx));
    quiet(&cx, &deck);
    update(&mut cx, &deck, |deck, cx| deck.select_objects(&["1".into()], cx));
    settle(&cx, |app| marked(app, &deck) == vec![1]);
    update(&mut cx, &deck, |deck, cx| deck.read_in(id, |worker, doc| worker.post_key(doc, 0, 770), |_, _, _, _| {}, cx));
    update(&mut cx, &deck, |deck, cx| deck.force_reread(id, cx));
    quiet(&cx, &deck);
    let refused = ask(&mut cx, &deck, &["replace", &file, "dog", "cat"]);
    assert!(refused.as_ref().is_err_and(|why| why.contains("inside a group")), "{refused:?}");
    assert_eq!(ask(&mut cx, &deck, &["get", &file]), Ok(after.replace(' ', "\t")), "nothing replaced");
}
