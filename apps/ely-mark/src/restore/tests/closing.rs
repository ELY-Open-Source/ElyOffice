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
}
