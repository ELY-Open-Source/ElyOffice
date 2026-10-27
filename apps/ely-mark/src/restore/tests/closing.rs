//! A refused save as a close waits on (X-08).

use ely_office::After;
use ely_office::prelude::*;
use gpui::{Entity, TestAppContext, VisualTestContext};

use crate::desk::Desk;
use crate::desk::tests::{open, start};

fn leaving(desk: &Entity<Desk>, cx: &mut VisualTestContext) -> bool {
    desk.read_with(cx, |desk, _| desk.keeping.leaving)
}

fn save_as(desk: &Entity<Desk>, id: u64, path: std::path::PathBuf, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        desk.update(cx, |desk, cx| desk.write_as(id, path, Some(After::Window), window, cx))
    });
    cx.run_until_parked();
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
    let ids: Vec<u64> = desk.read_with(cx, |desk, _| desk.core.tabs.iter().map(|tab| tab.id).collect());
    desk.update(cx, |desk, cx| desk.leave(cx));
    assert!(leaving(&desk, cx));
    // A's file is open in the other tab.
    save_as(&desk, ids[1], a.clone(), cx);
    assert!(!leaving(&desk, cx), "the close still waits; the list stays frozen");
    desk.update(cx, |desk, cx| desk.leave(cx));
    save_as(&desk, u64::MAX, files.path().join("c.md"), cx);
    assert!(!leaving(&desk, cx), "a gone tab left the close waiting");
    type Save = fn(&mut Desk, u64, Option<After>, &mut gpui::Window, &mut gpui::Context<Desk>);
    let saves: [(&str, Save); 2] = [("save", Desk::save), ("keep mine", Desk::keep_mine)];
    for (name, save) in saves {
        desk.update(cx, |desk, cx| desk.leave(cx));
        cx.update(|window, cx| desk.update(cx, |desk, cx| save(desk, u64::MAX, Some(After::Window), window, cx)));
        cx.run_until_parked();
        assert!(!leaving(&desk, cx), "{name} of a gone tab left the close waiting");
    }
}

#[gpui::test]
fn a_tab_closed_mid_export_stops_the_close_its_queued_save_held(cx: &mut TestAppContext) {
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
            desk.close_tab(id, cx);
            desk.proceed(After::Tab, window, cx);
        })
    });
    cx.run_until_parked();
    assert!(!leaving(&desk, cx), "the queued save's close was dropped");
    let later = files.path().join("later.md");
    std::fs::write(&later, "C\n").expect("later");
    open(&desk, &later, cx);
    desk.update(cx, |desk, cx| desk.keep(cx));
    cx.run_until_parked();
    let kept = std::fs::read_to_string(data.path().join("session.json")).expect("the list");
    assert!(kept.contains("later.md"), "{kept}");
}
