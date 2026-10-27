//! A save a close waits on, refused or orphaned (X-08).

use ely_office::After;
use ely_office::prelude::*;
use gpui::{Context, Entity, TestAppContext, VisualTestContext, Window};

use crate::desk::Desk;
use crate::desk::tests::{open, start};

type Save = fn(&mut Desk, u64, Option<After>, &mut Window, &mut Context<Desk>);

fn leaving(desk: &Entity<Desk>, cx: &mut VisualTestContext) -> bool {
    desk.read_with(cx, |desk, _| desk.keeping.leaving)
}

fn tabs(desk: &Entity<Desk>, cx: &mut VisualTestContext) -> Vec<u64> {
    desk.read_with(cx, |desk, _| desk.core.tabs.iter().map(|tab| tab.id).collect())
}

fn kept(data: &std::path::Path) -> String {
    std::fs::read_to_string(data.join("session.json")).expect("the list")
}

#[gpui::test]
fn a_refused_save_as_stops_the_close_it_held(cx: &mut TestAppContext) {
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    let (a, b) = (files.path().join("a.md"), files.path().join("b.md"));
    std::fs::write(&a, "A\n").expect("a");
    std::fs::write(&b, "B\n").expect("b");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &a, cx);
    open(&desk, &b, cx);
    let ids = tabs(&desk, cx);
    desk.update(cx, |desk, cx| desk.leave(cx));
    // A's file is open in the other tab.
    cx.update(|window, cx| {
        desk.update(cx, |desk, cx| desk.write_as(ids[1], a, Some(After::Window), window, cx))
    });
    cx.run_until_parked();
    assert!(!leaving(&desk, cx), "the close still waits; the list stays frozen");
    assert_eq!(tabs(&desk, cx), ids, "nothing closed");
}

/// A gone tab's save: the window's close goes on.
fn a_gone_tabs_save_lets_the_close_go_on(cx: &mut TestAppContext, save: Save) {
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    let a = files.path().join("a.md");
    std::fs::write(&a, "A\n").expect("a");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &a, cx);
    desk.update(cx, |desk, cx| desk.leave(cx));
    cx.update(|window, cx| desk.update(cx, |desk, cx| save(desk, u64::MAX, Some(After::Window), window, cx)));
    cx.run_until_parked();
    assert!(tabs(&desk, cx).is_empty(), "the close stopped at the gone tab");
    assert!(leaving(&desk, cx) && kept(data.path()).contains("a.md"));
}

#[gpui::test]
fn a_gone_tabs_save_goes_on(cx: &mut TestAppContext) {
    a_gone_tabs_save_lets_the_close_go_on(cx, Desk::save);
}

#[gpui::test]
fn a_gone_tabs_keep_mine_goes_on(cx: &mut TestAppContext) {
    a_gone_tabs_save_lets_the_close_go_on(cx, Desk::keep_mine);
}

#[gpui::test]
fn a_gone_tabs_save_as_goes_on(cx: &mut TestAppContext) {
    let elsewhere = |desk: &mut Desk, id, then, window: &mut Window, cx: &mut Context<Desk>| {
        let path = std::env::temp_dir().join("ely-mark-gone.md");
        desk.write_as(id, path, then, window, cx)
    };
    a_gone_tabs_save_lets_the_close_go_on(cx, elsewhere);
}

#[gpui::test]
fn a_tab_closed_mid_export_lets_the_window_close_go_on(cx: &mut TestAppContext) {
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    let b = files.path().join("b.md");
    std::fs::write(&b, "B\n").expect("b");
    let (desk, cx) = start(cx, data.path());
    desk.update(cx, |desk, cx| desk.new_from_text("untitled.md", "", cx));
    cx.run_until_parked();
    open(&desk, &b, cx);
    let (id, view) = desk.read_with(cx, |desk, _| (desk.core.tabs[0].id, desk.core.tabs[0].view.clone()));
    view.update(cx, |view, cx| view.adopt("Unsaved A\n".into(), cx));
    let first = files.path().join("first.md");
    cx.update(|window, cx| {
        desk.update(cx, |desk, cx| {
            desk.write_as(id, first, None, window, cx);
            assert!(desk.core.tabs[0].exporting, "the export runs");
            desk.leave(cx);
            // The window's close saves the tab: it queues.
            desk.save(id, Some(After::Window), window, cx);
            assert_eq!(desk.core.queued_saves.len(), 1);
            // The tab alone closes, discarded.
            desk.close_tab(id, cx);
            desk.proceed(After::Tab, window, cx);
        })
    });
    cx.run_until_parked();
    assert!(tabs(&desk, cx).is_empty(), "the window's close was dropped");
    let kept = kept(data.path());
    assert!(kept.contains("b.md") && !kept.contains("untitled.md"), "{kept}");
}

#[gpui::test]
fn a_second_close_of_the_window_keeps_the_first(cx: &mut TestAppContext) {
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    let (a, b) = (files.path().join("a.md"), files.path().join("b.md"));
    std::fs::write(&a, "A\n").expect("a");
    std::fs::write(&b, "B\n").expect("b");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &a, cx);
    open(&desk, &b, cx);
    let views: Vec<_> = desk.read_with(cx, |desk, _| desk.core.tabs.iter().map(|tab| (tab.id, tab.view.clone())).collect());
    for (_, view) in &views {
        view.update(cx, |view, cx| view.adopt("Changed\n".into(), cx));
    }
    cx.run_until_parked();
    let (id, other) = (views[0].0, views[1].0);
    cx.update(|window, cx| {
        desk.update(cx, |desk, cx| {
            desk.leave(cx);
            desk.save(id, Some(After::Window), window, cx);
            assert!(desk.exporting(id));
            // The close is asked again; its save of A queues.
            desk.leave(cx);
            desk.save(id, Some(After::Window), window, cx);
            assert_eq!(desk.core.queued_saves.len(), 1);
        })
    });
    cx.run_until_parked();
    let asks_b = desk.read_with(cx, |desk, _| {
        matches!(desk.core.dialog, Some(ely_office::Dialog::Unsaved { tab, after: After::Window }) if tab == other)
    });
    assert_eq!(tabs(&desk, cx), [other]);
    assert!(asks_b && leaving(&desk, cx), "the running close was stopped");
    let kept = kept(data.path());
    assert!(kept.contains("a.md") && kept.contains("b.md"), "{kept}");
}
