//! Inserted links: no false error, kept in PPTX (SHOW-03).

use ely_office::prelude::*;
use ely_ui::Root;
use gpui::{Entity, TestAppContext};

use super::objects::{blank, objects};
use super::slides::{in_window, type_in};
use super::{quiet, settle, undo_steps, update};
use crate::deck::Deck;
use crate::dialog::Dialog;

const ADDRESS: &str = "https://example.com/plan";
/// VCL's Ctrl (Cmd on macOS).
const MOD1: i32 = 0x2000;
const ESCAPE: i32 = 1281;

/// A blank deck whose window root keeps the toasts.
fn rooted(cx: &mut TestAppContext, dir: &std::path::Path) -> Option<(Entity<Deck>, Entity<Root>)> {
    let deck = blank(cx, dir)?;
    let root = root_for(cx, &deck);
    Some((deck, root))
}

/// The window root that keeps `deck`'s toasts.
pub(super) fn root_for(cx: &mut TestAppContext, deck: &Entity<Deck>) -> Entity<Root> {
    in_window(cx, deck, |deck, window, cx| {
        let this = cx.entity();
        let root = ely_ui::document_root(ely_platform::AppId::Show, this.into(), &ely_ui::commands::FILE, Vec::new(), window, cx);
        deck.core.root = root.downgrade();
        root
    })
}

/// Runs `send` on the active deck's kernel document.
pub(super) fn kernel(cx: &mut TestAppContext, deck: &Entity<Deck>, send: impl FnOnce(&mut ely_lok::Worker, &ely_lok::Doc)) {
    cx.update(|app| {
        let deck = deck.read(app);
        let doc = deck.active_tab().expect("a tab").doc.clone();
        send(&mut deck.core.worker.lock().expect("the worker lock"), &doc);
    });
}

/// Clicks the title at `x`, then types `typed`.
fn on_title(cx: &mut TestAppContext, deck: &Entity<Deck>, (x, modifiers): (i32, i32), typed: &str) {
    kernel(cx, deck, |worker, doc| {
        for kind in [0, 1] {
            let click = ely_lok::Mouse { kind, x, y: 1100, count: 1, buttons: 1, modifiers };
            worker.post_mouse(doc, click).expect("clicks");
        }
        for c in typed.chars() {
            worker.post_key(doc, c as u32, 0).expect("types");
        }
    });
}

/// The edited text box's whole text, copied as plain text.
pub(super) fn edited_text(cx: &mut TestAppContext, deck: &Entity<Deck>) -> String {
    let mut copied = Vec::new();
    kernel(cx, deck, |worker, doc| {
        worker.run(doc, "edit.select_all", &[]).expect("selects");
        copied = worker.copy(doc).expect("copies");
    });
    let (_, text) = copied.into_iter().find(|(mime, _)| mime.starts_with("text/plain")).expect("plain text");
    String::from_utf8(text).expect("UTF-8")
}

/// Fills and sends the hyperlink dialog.
fn insert_link(cx: &mut TestAppContext, deck: &Entity<Deck>, (words, address): (&str, &str)) {
    in_window(cx, deck, |deck, window, cx| deck.ask_link(window, cx));
    let fields = cx.update(|app| match &deck.read(app).dialog {
        Some(Dialog::Link { text, url, .. }) => Some((text.clone(), url.clone())),
        _ => None,
    });
    let (text, url) = fields.expect("the link dialog");
    type_in(cx, &text, words);
    type_in(cx, &url, address);
    update(cx, deck, |deck, cx| deck.link_asked(cx));
    quiet(cx, deck);
    cx.run_until_parked();
}

pub(super) fn no_toasts(cx: &mut TestAppContext, root: &Entity<Root>) {
    let toasts = cx.update(|app| root.read(app).toasts(app));
    assert!(toasts.is_empty(), "{toasts:?}");
}

#[test]
fn a_link_in_edited_text_reports_no_error_and_survives_pptx() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((deck, root)) = rooted(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    on_title(&mut cx, &deck, (7900, 0), "Plan ");
    let caret = |app: &gpui::App| deck.read(app).active_tab().is_some_and(|tab| tab.view.read(app).caret());
    settle(&cx, caret);
    quiet(&cx, &deck);
    insert_link(&mut cx, &deck, (" the site ", ADDRESS));
    no_toasts(&mut cx, &root);
    assert_eq!(edited_text(&mut cx, &deck), "Plan the site");
    kernel(&mut cx, &deck, |worker, doc| worker.post_key(doc, 0, ESCAPE).expect("leaves the text"));
    settle(&cx, |app| !caret(app));
    assert_eq!(undo_steps(&mut cx, &deck), 1, "the edit with its link is one step");
    let copy = dir.path().join("copy.pptx");
    let id = cx.update(|app| deck.read(app).core.active()).expect("a tab");
    in_window(&mut cx, &deck, |deck, window, cx| deck.write_as(id, copy.clone(), None, window, cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.name == "copy.pptx"));
    quiet(&cx, &deck);
    update(&mut cx, &deck, |deck, cx| deck.close_tab(id, cx));
    settle(&cx, |app| deck.read(app).core.tabs.is_empty());
    update(&mut cx, &deck, |deck, cx| deck.open_with(copy.clone(), String::new(), cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| !tab.extra.slides.list.is_empty()));
    quiet(&cx, &deck);
    // Ctrl+click on the linked words opens the address.
    on_title(&mut cx, &deck, (9000, MOD1), "");
    settle(&cx, |_| cx.opened_url().is_some());
    assert_eq!(cx.opened_url().as_deref(), Some(ADDRESS));
    no_toasts(&mut cx, &root);
}

#[test]
fn a_link_outside_text_is_a_new_text_box_in_one_step() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((deck, root)) = rooted(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    quiet(&cx, &deck);
    let shapes = |app: &gpui::App| objects(app, &deck).map(|objects| objects.objects.len());
    let (before, steps) = (cx.update(|app| shapes(app)), undo_steps(&mut cx, &deck));
    insert_link(&mut cx, &deck, ("", ADDRESS));
    no_toasts(&mut cx, &root);
    assert_eq!(undo_steps(&mut cx, &deck), steps + 1, "one step");
    settle(&cx, |app| shapes(app) > before);
}

#[test]
fn a_clicked_local_or_script_link_opens_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((deck, root)) = rooted(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let refused = cx.update(|app| Deck::text(app, "office.link.refused", &[]));
    let caret = |app: &gpui::App| deck.read(app).active_tab().is_some_and(|tab| tab.view.read(app).caret());
    let unsafe_links = ["file:///etc/passwd", "vnd.sun.star.script:Standard.Module1.Main?language=Basic&location=document", "macro:///Tools.Misc.Run"];
    for address in unsafe_links {
        on_title(&mut cx, &deck, (7900, 0), "");
        settle(&cx, caret);
        kernel(&mut cx, &deck, |worker, doc| worker.run(doc, "edit.select_all", &[]).expect("selects"));
        insert_link(&mut cx, &deck, ("the link", address));
        assert_eq!(edited_text(&mut cx, &deck), "the link");
        // Leave the text, then drop the selection.
        kernel(&mut cx, &deck, |worker, doc| {
            worker.post_key(doc, 0, ESCAPE).expect("leaves the text");
            worker.post_key(doc, 0, ESCAPE).expect("deselects");
        });
        settle(&cx, |app| !caret(app));
        quiet(&cx, &deck);
        let said = |app: &gpui::App| root.read(app).toasts(app).iter().filter(|(text, _)| *text == refused).count();
        // Toasts leave on the test clock: this click's own.
        settle(&cx, |app| said(app) == 0);
        on_title(&mut cx, &deck, (7900, MOD1), "");
        settle(&cx, |app| said(app) > 0);
        assert_eq!(cx.opened_url(), None, "{address}");
    }
}
