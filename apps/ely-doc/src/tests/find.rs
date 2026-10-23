use ely_gpui_component::forms::TextInput;
use ely_office::{FindReplace, Finder, prelude::*};
use gpui::{AppContext as _, Entity, TestAppContext};

use crate::docs::Docs;
use crate::tests::{blank, docs, settle, settled, text_of, type_into, with_window, written};

pub(crate) fn finder<R>(cx: &TestAppContext, docs: &Entity<Docs>, read: impl Fn(&Finder) -> R) -> R {
    cx.update(|app| {
        read(
            docs.read(app)
                .core
                .finder
                .as_ref()
                .expect("the finder is open"),
        )
    })
}

pub(crate) fn total(app: &gpui::App, docs: &Entity<Docs>) -> Option<usize> {
    docs.read(app)
        .core
        .finder
        .as_ref()
        .map(|finder| finder.all.len())
}

#[test]
fn find_counts_steps_replaces_and_counts_words() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "cat Cat cat dog");
    settle(&cx, |app| {
        docs.read(app)
            .active_tab()
            .and_then(|tab| tab.extra.counts)
            .is_some_and(|counts| counts.words == 4)
    });
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_find(&FindReplace, window, cx)
    });
    let query = finder(&cx, &docs, |finder| finder.query.clone());
    cx.update(|app| query.update(app, |query, cx| query.set_text("cat", cx)));
    settle(&cx, |app| total(app, &docs) == Some(3));
    cx.update(|app| {
        docs.update(app, |docs, cx| {
            docs.core.finder.as_mut().expect("open").options.case = true;
            docs.count_matches(cx);
        })
    });
    settle(&cx, |app| total(app, &docs) == Some(2));
    cx.update(|app| docs.update(app, |docs, cx| docs.step(true, cx)));
    settle(&cx, |app| {
        docs.read(app)
            .core
            .finder
            .as_ref()
            .is_some_and(|finder| finder.current == Some(0))
    });
    with_window(&mut cx, &docs, |docs, window, cx| {
        let finder = docs.core.finder.as_mut().expect("open");
        finder.replace = Some(cx.new(|cx| TextInput::new(window, cx)));
    });
    let with = finder(&cx, &docs, |finder| {
        finder.replace.clone().expect("a field")
    });
    cx.update(|app| with.update(app, |with, cx| with.set_text("cow", cx)));
    cx.update(|app| docs.update(app, |docs, cx| docs.replace(false, cx)));
    settle(&cx, |app| total(app, &docs) == Some(1));
    cx.update(|app| docs.update(app, |docs, cx| docs.replace(true, cx)));
    settle(&cx, |app| total(app, &docs) == Some(0));
    settled(&cx, &docs);
    let path = dir.path().join("found.odt");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, path.clone(), None, window, cx)
    });
    written(&cx, &path, None);
    let text = text_of(&mut cx, &docs, &path);
    assert!(text.contains("cow Cat cow dog"), "the cased matches go");
}

#[test]
fn edits_mark_the_count_and_steps_scroll_to_far_matches() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let filler = "<text:p>filler</text:p>".repeat(100);
    let body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text"><office:body><office:text><text:p>a b c</text:p>{filler}<text:p>needle</text:p></office:text></office:body></office:document>"#
    );
    let path = dir.path().join("long.fodt");
    std::fs::write(&path, body).expect("the file writes");
    with_window(&mut cx, &docs, |docs, _, cx| {
        docs.open_file(path.clone(), cx)
    });
    settle(&cx, |app| docs.read(app).core.tabs.len() == 1);
    settled(&cx, &docs);
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_find(&FindReplace, window, cx)
    });
    let query = finder(&cx, &docs, |finder| finder.query.clone());
    cx.update(|app| query.update(app, |query, cx| query.set_text(" ", cx)));
    settle(&cx, |app| total(app, &docs) == Some(2));
    cx.update(|app| query.update(app, |query, cx| query.set_text("needle", cx)));
    settle(&cx, |app| total(app, &docs) == Some(1));
    cx.update(|app| docs.update(app, |docs, cx| docs.step(true, cx)));
    settle(&cx, |app| {
        let tab = docs.read(app).active_tab().expect("a tab");
        tab.view.read(app).scroll().1 > 20_000
    });
    type_into(&mut cx, &docs, "needle needle");
    settle(&cx, |app| {
        docs.read(app)
            .core
            .finder
            .as_ref()
            .is_some_and(|finder| finder.stale)
    });
    cx.update(|app| docs.update(app, |docs, cx| docs.step(true, cx)));
    settle(&cx, |app| {
        let finder = docs.read(app).core.finder.as_ref().expect("open");
        !finder.stale && finder.all.len() == 2
    });
}

#[test]
fn a_step_asked_during_a_count_waits_for_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "cat cat cat");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_find(&FindReplace, window, cx)
    });
    let query = finder(&cx, &docs, |finder| finder.query.clone());
    cx.update(|app| query.update(app, |query, cx| query.set_text("cat", cx)));
    cx.update(|app| docs.update(app, |docs, cx| docs.step(true, cx)));
    cx.update(|app| docs.update(app, |docs, cx| docs.step(true, cx)));
    assert_eq!(
        finder(&cx, &docs, |finder| finder.queued.len()),
        2,
        "both steps wait for the count"
    );
    settle(&cx, |app| {
        docs.read(app)
            .core
            .finder
            .as_ref()
            .is_some_and(|finder| finder.all.len() == 3 && finder.current == Some(1))
    });
    cx.update(|app| query.update(app, |query, cx| query.set_text("ca", cx)));
    cx.update(|app| query.update(app, |query, cx| query.set_text("", cx)));
    settle(&cx, |app| {
        docs.read(app)
            .core
            .finder
            .as_ref()
            .is_some_and(|finder| finder.pending.is_empty())
    });
    assert_eq!(
        finder(&cx, &docs, |finder| finder.all.len()),
        0,
        "a cleared query voids the count on its way"
    );
}
