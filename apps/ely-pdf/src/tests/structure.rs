//! Links, bookmarks, properties, attachments, OCR from the app (C11.5).

use ely_pdf_engine::{
    Worker,
    sample::{Extras, pdf, pdf_with, scan},
};
use gpui::{Entity, Modifiers, MouseButton, ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, point, px};

use super::input::over;
use super::{frames, settle, shown, windowed};
use crate::annotate::Pick;
use crate::content::{Aim, Tool};
use crate::marking::Change;
use crate::view::PdfView;
use crate::viewer::Viewer;

/// Two pages, a bookmark, a link to page two.
fn opened<'a>(cx: &'a mut TestAppContext, dir: &std::path::Path) -> (Entity<Viewer>, &'a mut VisualTestContext, Entity<PdfView>) {
    let path = dir.join("doc.pdf");
    let pages: [(f32, f32, &[&str]); 2] = [(612.0, 792.0, &["Page one"]), (612.0, 792.0, &["Page two"])];
    std::fs::write(&path, pdf_with(&pages, &[("One", 0, &[])], &Extras { links: &[(0, 1)], ..Extras::default() })).expect("writes");
    let (viewer, cx) = windowed(cx, dir, &[path], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > gpui::px(100.0));
    cx.update(|_, app| view.update(app, |view, cx| {
        view.zoom = crate::layout::Zoom::Scale(1.0);
        view.mode = crate::layout::Mode::Single;
        cx.notify();
    }));
    settle(cx, |app| view.read(app).links.contains_key(&0));
    (viewer, cx, view)
}

fn press(cx: &mut VisualTestContext, view: &Entity<PdfView>, from: (f32, f32), to: (f32, f32)) {
    frames(cx);
    let (a, b) = (over(cx, view, 0, from), over(cx, view, 0, to));
    cx.simulate_mouse_down(a, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(b, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(b, MouseButton::Left, Modifiers::none());
}

fn landed(cx: &mut VisualTestContext, view: &Entity<PdfView>, edits: u64) {
    settle(cx, |app| view.read(app).edited == edits && view.read(app).settled() && view.read(app).links.contains_key(&0));
}

fn send(cx: &mut VisualTestContext, viewer: &Entity<Viewer>, change: Change) {
    frames(cx);
    cx.update(|_, app| viewer.update(app, |viewer, cx| {
        let shown = viewer.note_panel.read(cx).shown.expect("the panel follows the pick");
        viewer.change_note(shown, change, cx)
    }));
}

#[test]
fn a_click_follows_a_link_and_the_link_tool_adds_retargets_and_deletes() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = opened(&mut cx, dir.path());
    // The sample link covers 72..200 across, 692..720 down.
    press(cx, &view, (100.0, 700.0), (100.0, 700.0));
    settle(cx, |app| view.read(app).current == 1);
    cx.update(|_, app| view.update(app, |view, cx| view.show(0, 0.0, cx)));
    cx.update(|_, app| view.update(app, |view, cx| view.set_tool(Tool::Link, cx)));
    press(cx, &view, (72.0, 300.0), (200.0, 320.0));
    assert!(matches!(cx.update(|_, app| view.read(app).aim), Some(Aim::Note(Pick::NewLink { .. }))));
    send(cx, &viewer, Change::Target("javascript:x".into()));
    frames(cx);
    assert_eq!(cx.update(|_, app| view.read(app).edited), 0, "a bad target is refused");
    send(cx, &viewer, Change::Target("https://example.com/".into()));
    landed(cx, &view, 1);
    let links = cx.update(|_, app| view.read(app).links[&0].clone());
    assert_eq!((links.len(), links[1].uri.as_str()), (2, "https://example.com/"));
    press(cx, &view, (100.0, 310.0), (100.0, 310.0));
    assert!(matches!(cx.update(|_, app| view.read(app).aim), Some(Aim::Note(Pick::Link { index: 1, .. }))));
    send(cx, &viewer, Change::Target("2".into()));
    landed(cx, &view, 2);
    assert_eq!(cx.update(|_, app| view.read(app).links[&0][1].page), Some(1));
    press(cx, &view, (100.0, 310.0), (100.0, 310.0));
    send(cx, &viewer, Change::Delete);
    landed(cx, &view, 3);
    assert_eq!(cx.update(|_, app| view.read(app).links[&0].len()), 1);
}

#[test]
fn bookmarks_add_after_the_pick_rename_and_delete() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = opened(&mut cx, dir.path());
    let titles = |cx: &mut VisualTestContext| cx.update(|_, app| view.read(app).marks.iter().map(|m| m.title.clone()).collect::<Vec<_>>());
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.ask_bookmark(true, window, cx)));
    let typed = cx.update(|_, app| viewer.read(app).naming.as_ref().expect("asked").field.read(app).text().to_string());
    let label = cx.update(|_, app| Viewer::text(app, "pdf.page.label", &[("n", "1")]).to_string());
    assert_eq!(typed, label, "named for the page");
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.name_bookmark("Start".into(), cx)));
    landed(cx, &view, 1);
    assert_eq!(titles(cx), ["One", "Start"]);
    cx.update(|_, app| view.update(app, |view, _| view.mark_pick = Some(0)));
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.ask_bookmark(false, window, cx)));
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.name_bookmark("First".into(), cx)));
    landed(cx, &view, 2);
    assert_eq!(titles(cx), ["First", "Start"]);
    cx.update(|_, app| view.update(app, |view, _| view.mark_pick = Some(1)));
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.delete_bookmark(cx)));
    landed(cx, &view, 3);
    assert_eq!(titles(cx), ["First"]);
}

#[test]
fn a_long_bookmark_list_scrolls_to_its_last_mark() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = dir.path().join("long.pdf");
    let titles: Vec<String> = (0..100).map(|n| format!("Chapter {n}")).collect();
    let marks: Vec<_> = titles.iter().map(|title| (title.as_str(), 0, &[][..])).collect();
    std::fs::write(&path, pdf(&[(612.0, 792.0, &["one"])], &marks)).expect("writes");
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).marks.len() == 100);
    cx.update(|_, app| viewer.read(app).side.clone()).update(cx, |side, cx| {
        side.pane = crate::side::Pane::Marks;
        cx.notify();
    });
    frames(cx);
    let before = cx.debug_bounds("pdf-marks").expect("the bookmarks are drawn");
    let window = cx.update(|window, _| window.viewport_size());
    assert!(before.bottom() > window.height, "the list is longer than the window");
    let at = point(before.left() + px(20.0), px(300.0));
    for _ in 0..20 {
        cx.simulate_event(ScrollEvent { position: at, delta: ScrollDelta::Pixels(point(px(0.0), px(-1000.0))), ..Default::default() });
        frames(cx);
    }
    let after = cx.debug_bounds("pdf-marks").expect("the bookmarks are drawn");
    assert!(after.bottom() <= window.height, "the last mark is in view: {after:?}");
}

#[test]
fn properties_show_and_set_what_changed() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = opened(&mut cx, dir.path());
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.ask_properties(window, cx)));
    settle(cx, |app| viewer.read(app).props.is_some());
    cx.update(|_, app| viewer.update(app, |viewer, cx| {
        let field = viewer.props.as_ref().expect("open").fields[0].clone();
        field.update(cx, |field, cx| field.set_text("Report", cx));
        viewer.set_properties(cx)
    }));
    landed(cx, &view, 1);
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.ask_properties(window, cx)));
    settle(cx, |app| viewer.read(app).props.is_some());
    assert_eq!(cx.update(|_, app| viewer.read(app).props.as_ref().expect("open").read.title.clone()), Some("Report".into()));
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.set_properties(cx)));
    frames(cx);
    assert_eq!(cx.update(|_, app| view.read(app).edited), 1, "nothing changed, no step");
}

#[test]
fn attachments_list_save_and_remove() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = opened(&mut cx, dir.path());
    let file = dir.path().join("notes.txt");
    std::fs::write(&file, "notes").expect("writes");
    cx.update(|_, app| view.update(app, |view, cx| view.edit("attaching", move |w, doc| w.attach(doc, "notes.txt", &file), cx)));
    landed(cx, &view, 1);
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.side.update(cx, |side, cx| {
        side.pane = crate::side::Pane::Files;
        cx.notify();
    })));
    settle(cx, |app| view.read(app).files.as_ref().is_some_and(|files| files.len() == 1));
    let out = dir.path().join("saved.txt");
    let id = cx.update(|_, app| viewer.read(app).core.tabs[0].id);
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_attachment(id, "n:notes.txt".into(), out.clone(), cx)));
    settle(cx, |_| out.is_file());
    assert_eq!(std::fs::read(&out).expect("reads"), b"notes");
    cx.update(|_, app| view.update(app, |view, _| view.file_pick = Some("n:notes.txt".into())));
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.remove_attachment(cx)));
    settle(cx, |app| view.read(app).files.as_ref().is_some_and(Vec::is_empty));
}

/// A scan of two English lines.
fn scanned(dir: &std::path::Path) -> std::path::PathBuf {
    let source = dir.join("text.pdf");
    std::fs::write(&source, pdf(&[(612.0, 792.0, &["Hello world", "Second line"])], &[])).expect("writes");
    let mut worker = Worker::spawn().expect("the worker");
    let doc = worker.open(&source).expect("opens").doc;
    let s: f32 = 200.0 / 72.0;
    let (w, h) = ((612.0 * s).ceil() as i32, (792.0 * s).ceil() as i32);
    let bgra = worker.render(doc, 0, (s, 0), [0, 0, w, h]).expect("draws").bgra;
    let gray: Vec<u8> = bgra.chunks_exact(4).map(|p| ((u16::from(p[0]) + u16::from(p[1]) + u16::from(p[2])) / 3) as u8).collect();
    let path = dir.join("scan.pdf");
    std::fs::write(&path, scan((612.0, 792.0), (w as u32, h as u32), &gray)).expect("writes");
    path
}

#[test]
fn ocr_reads_the_pages_then_lays_their_text_in_one_step() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = scanned(dir.path());
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    cx.update(|window, app| viewer.update(app, |viewer, cx| {
        viewer.ask_ocr(window, cx);
        viewer.start_ocr(cx);
    }));
    assert!(cx.update(|_, app| viewer.read(app).ocr.as_ref().is_some_and(|ocr| ocr.run.is_some())), "reading");
    settle(cx, |app| viewer.read(app).ocr.is_none() && view.read(app).edited == 1);
    let text = cx.update(|_, app| {
        let doc = view.read(app).doc;
        viewer.read(app).core.worker.lock().expect("the worker lock").get().expect("started").text(doc, 0).expect("text")
    });
    let read: String = text.iter().map(|c| c.c).collect();
    assert!(read.contains("Hello world"), "{read}");
    // A second run skips the page that has text now.
    cx.update(|window, app| viewer.update(app, |viewer, cx| {
        viewer.ask_ocr(window, cx);
        viewer.start_ocr(cx);
    }));
    settle(cx, |app| viewer.read(app).ocr.is_none());
    frames(cx);
    assert_eq!(cx.update(|_, app| view.read(app).edited), 1, "nothing to lay");
}

#[test]
fn a_cancelled_ocr_lays_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = scanned(dir.path());
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    cx.update(|window, app| viewer.update(app, |viewer, cx| {
        viewer.ask_ocr(window, cx);
        viewer.start_ocr(cx);
        viewer.ocr = None;
    }));
    for _ in 0..20 {
        frames(cx);
    }
    assert_eq!(cx.update(|_, app| view.read(app).edited), 0);
}

#[test]
fn a_cancelled_run_cannot_touch_the_next() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = scanned(dir.path());
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    let tab = cx.update(|_, app| viewer.read(app).core.tabs[0].id);
    let first = cx.update(|window, app| viewer.update(app, |viewer, cx| {
        viewer.ask_ocr(window, cx);
        viewer.start_ocr(cx);
        viewer.ocr = None;
        viewer.ask_ocr(window, cx);
        viewer.start_ocr(cx);
        viewer.ocr_runs - 1
    }));
    cx.update(|_, app| viewer.update(app, |viewer, cx| {
        viewer.ocr_read((tab, first, 0), Err(std::io::Error::other("from the first run")), cx)
    }));
    assert!(cx.update(|_, app| viewer.read(app).ocr.is_some()), "the second run goes on");
    settle(cx, |app| viewer.read(app).ocr.is_none() && view.read(app).edited == 1);
}

#[test]
fn a_refused_text_layer_reports_no_success() {
    use ely_gpui_component::primitives::Severity;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = scanned(dir.path());
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    // The window's toasts live on its root.
    let root = cx.update(|window, app| {
        let root = ely_ui::document_root(ely_platform::AppId::Pdf, viewer.clone().into(), &ely_ui::commands::FILE, Vec::new(), window, app);
        viewer.update(app, |viewer, _| viewer.core.root = root.downgrade());
        root
    });
    let tab = cx.update(|_, app| viewer.read(app).core.tabs[0].id);
    let run = cx.update(|window, app| viewer.update(app, |viewer, cx| {
        viewer.ask_ocr(window, cx);
        viewer.start_ocr(cx);
        viewer.ocr_runs - 1
    }));
    // An edit still runs when the text is laid.
    cx.update(|_, app| view.update(app, |view, _| view.editing = true));
    let line = ely_pdf_engine::OcrLine { rect: [72.0, 700.0, 300.0, 720.0], text: "Hello".into() };
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.ocr_read((tab, run, 0), Ok(Some(vec![line])), cx)));
    settle(cx, |app| viewer.read(app).ocr.is_none());
    let toasts = cx.update(|_, app| root.read(app).toasts(app));
    assert!(toasts.iter().any(|(_, severity)| *severity == Some(Severity::Warning)), "{toasts:?}");
    assert!(!toasts.iter().any(|(_, severity)| *severity == Some(Severity::Success)), "{toasts:?}");
}
