//! Marking, removing and cleaning from the app (C11.6).

use ely_pdf_engine::{Cleaning, Worker, sample::{Extras, pdf_with}};
use gpui::{Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext};

use super::input::over;
use super::{frames, settle, shown, windowed};
use crate::content::Tool;
use crate::redact::Removal;
use crate::view::PdfView;
use crate::viewer::Viewer;

/// Two lines and a title, actual size.
fn opened<'a>(cx: &'a mut TestAppContext, dir: &std::path::Path) -> (Entity<Viewer>, &'a mut VisualTestContext, Entity<PdfView>) {
    let path = dir.join("doc.pdf");
    std::fs::write(&path, pdf_with(&[(612.0, 792.0, &["Hello world", "Second line"])], &[], &Extras::default())).expect("writes");
    let (viewer, cx) = windowed(cx, dir, &[path], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > gpui::px(100.0));
    cx.update(|_, app| view.update(app, |view, cx| {
        view.zoom = crate::layout::Zoom::Scale(1.0);
        cx.notify();
    }));
    frames(cx);
    (viewer, cx, view)
}

fn drag(cx: &mut VisualTestContext, view: &Entity<PdfView>, from: (f32, f32), to: (f32, f32)) {
    frames(cx);
    let (a, b) = (over(cx, view, 0, from), over(cx, view, 0, to));
    cx.simulate_mouse_down(a, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(b, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(b, MouseButton::Left, Modifiers::none());
}

fn text_of(path: &std::path::Path) -> String {
    let mut worker = Worker::spawn().expect("the worker");
    let doc = worker.open(path).expect("opens").doc;
    worker.text(doc, 0).expect("text").iter().map(|c| c.c).collect()
}

#[test]
fn areas_mark_and_a_click_unmarks() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (_, cx, view) = opened(&mut cx, dir.path());
    cx.update(|_, app| view.update(app, |view, cx| view.set_tool(Tool::RedactArea, cx)));
    drag(cx, &view, (100.0, 300.0), (200.0, 350.0));
    assert_eq!(cx.update(|_, app| view.read(app).redactions.len()), 1);
    drag(cx, &view, (150.0, 320.0), (150.0, 320.0));
    assert!(cx.update(|_, app| view.read(app).redactions.is_empty()), "a click inside takes it back");
}

#[test]
fn marked_text_is_removed_into_a_new_file() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = opened(&mut cx, dir.path());
    cx.update(|_, app| view.update(app, |view, cx| view.set_tool(Tool::RedactText, cx)));
    drag(cx, &view, (73.0, 66.0), (110.0, 66.0));
    settle(cx, |app| !view.read(app).redactions.is_empty());
    let out = dir.path().join("out.pdf");
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.ask_redact(cx)));
    let asked = cx.update(|_, app| viewer.update(app, |viewer, _| viewer.removal.take().expect("asked")));
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_removal(asked, out.clone(), cx)));
    settle(cx, |_| out.is_file());
    let left = text_of(&out);
    assert!(!left.contains("Hello") && left.contains("Second line"), "{left}");
    assert_eq!(cx.update(|_, app| view.read(app).edited), 0, "the open document is untouched");
}

#[test]
fn an_edit_drops_the_marks_and_none_asks_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = opened(&mut cx, dir.path());
    cx.update(|_, app| view.update(app, |view, cx| view.mark_regions(0, vec![[72.0, 60.0, 120.0, 80.0]], None, cx)));
    cx.update(|_, app| view.update(app, |view, cx| view.edit("pdf.doing.turn", |w, doc| w.rotate_pages(doc, &[0], 90), cx)));
    settle(cx, |app| view.read(app).edited == 1);
    assert!(cx.update(|_, app| view.read(app).redactions.is_empty()), "pages may have moved");
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.ask_redact(cx)));
    assert!(cx.update(|_, app| viewer.read(app).removal.is_none()));
}

#[test]
fn cleaning_counts_first_then_writes_a_clean_file() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = opened(&mut cx, dir.path());
    cx.update(|_, app| view.update(app, |view, cx| view.edit("pdf.doing.properties", |w, doc| w.set_properties(doc, &[("Title", "Plan")]), cx)));
    settle(cx, |app| view.read(app).edited == 1);
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.ask_clean(cx)));
    settle(cx, |app| viewer.read(app).removal.is_some());
    let (tab, version, reach) = cx.update(|_, app| match viewer.read(app).removal.as_ref() {
        Some(Removal::Clean { tab, version, reach, .. }) => (*tab, *version, reach.clone()),
        _ => panic!("a cleaning"),
    });
    assert!(reach.meta >= 1, "{reach:?}");
    let out = dir.path().join("clean.pdf");
    let what = Cleaning { meta: true, ..Cleaning::default() };
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_removal(Removal::Clean { tab, version, reach, what }, out.clone(), cx)));
    settle(cx, |_| out.is_file());
    let mut worker = Worker::spawn().expect("the worker");
    let doc = worker.open(&out).expect("opens").doc;
    assert_eq!(worker.properties(doc).expect("reads").title, None);
}

#[test]
fn marks_changed_after_asking_write_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = opened(&mut cx, dir.path());
    cx.update(|_, app| view.update(app, |view, cx| view.mark_regions(0, vec![[72.0, 52.0, 120.0, 78.0]], None, cx)));
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.ask_redact(cx)));
    let asked = cx.update(|_, app| viewer.update(app, |viewer, _| viewer.removal.take().expect("asked")));
    cx.update(|_, app| view.update(app, |view, cx| view.mark_regions(0, vec![[72.0, 76.0, 200.0, 102.0]], None, cx)));
    let out = dir.path().join("out.pdf");
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_removal(asked, out.clone(), cx)));
    for _ in 0..10 {
        frames(cx);
    }
    assert!(!out.exists(), "the marks are not what was confirmed");
}

#[test]
fn a_scan_for_a_tab_left_behind_opens_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (a, b) = (dir.path().join("a.pdf"), dir.path().join("b.pdf"));
    for path in [&a, &b] {
        std::fs::write(path, pdf_with(&[(612.0, 792.0, &["Hello"])], &[], &Extras::default())).expect("writes");
    }
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[a, b], true);
    let first = cx.update(|_, app| viewer.read(app).core.tabs[0].id);
    cx.update(|_, app| viewer.update(app, |viewer, cx| {
        viewer.core.set_active(Some(first));
        viewer.ask_clean(cx);
        viewer.core.set_active(Some(viewer.core.tabs[1].id));
    }));
    for _ in 0..20 {
        frames(cx);
    }
    assert!(cx.update(|_, app| viewer.read(app).removal.is_none()), "another tab is in front");
}
