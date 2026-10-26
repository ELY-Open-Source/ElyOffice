//! The viewer against the real worker and generated PDFs (C11.1).

use std::sync::{Arc, Mutex};

use ely_office::{Opening as _, Tabs as _};
use ely_pdf_engine::{Worker, sample::pdf};
use gpui::{Entity, TestAppContext, VisualTestContext};

use crate::engine::Mupdf;
use crate::layout::Mode;
use crate::select::Selection;
use crate::view::PdfView;
use crate::viewer::Viewer;

mod editing;
mod input;
mod notes;
mod organize;
mod output;
mod protect;
mod reads;
mod removal;
mod row;
mod sign;
mod structure;
mod view;
mod visible;

/// Steps and redraws until `done`, at most ten seconds.
pub(crate) fn settle(cx: &mut VisualTestContext, done: impl Fn(&gpui::App) -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        // Bounded: a busy loop must fail here, not hang.
        for _ in 0..100 {
            if !cx.executor().tick() {
                break;
            }
        }
        cx.update(|window, _| window.refresh());
        if cx.update(|_, app| done(app)) {
            return;
        }
    }
    panic!("the state never settled");
}

/// A few redraws with the work they start.
pub(crate) fn frames(cx: &mut VisualTestContext) {
    for _ in 0..5 {
        cx.update(|window, _| window.refresh());
        for _ in 0..20 {
            cx.executor().tick();
        }
    }
}

pub(crate) fn file(dir: &std::path::Path, name: &str, pages: &[(f32, f32, &[&str])]) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, pdf(pages, &[])).expect("writes");
    path
}

/// A viewer drawn in a window, `paths` opened in order.
pub(crate) fn windowed<'a>(
    cx: &'a mut TestAppContext,
    dir: &std::path::Path,
    paths: &[std::path::PathBuf],
    worker: bool,
) -> (Entity<Viewer>, &'a mut VisualTestContext) {
    cx.update(ely_gpui_component::init_for_tests);
    cx.update(|cx| {
        ely_ui::settings::install_defaults(cx);
        ely_ui::i18n::install(cx);
    });
    cx.update(crate::keys::bind);
    cx.update(|cx| {
        // As document_root does: menus before the window.
        let menus = crate::keys::menus(cx);
        let file = &ely_ui::commands::FILE;
        cx.set_menus(ely_ui::commands::menus(ely_platform::AppId::Pdf, file, true, menus, cx));
    });
    let held = match worker {
        true => Mupdf::with(Worker::spawn().expect("the worker")),
        false => Mupdf::default(),
    };
    let work = dir.join("work");
    std::fs::create_dir_all(&work).expect("a work dir");
    let (viewer, vcx) = cx.add_window_view(move |window, cx| {
        let mut viewer = Viewer::new(Arc::new(Mutex::new(held)), work, cx);
        viewer.core.window = Some(window.window_handle());
        viewer
    });
    for path in paths {
        let count = vcx.update(|_, app| viewer.read(app).core.tabs.len());
        vcx.update(|_, app| viewer.update(app, |viewer, cx| viewer.open_file(path.clone(), cx)));
        settle(vcx, |app| viewer.read(app).core.tabs.len() == count + 1);
    }
    (viewer, vcx)
}

/// The active tab's view.
pub(crate) fn shown(cx: &mut VisualTestContext, viewer: &Entity<Viewer>) -> Entity<PdfView> {
    cx.update(|_, app| viewer.read(app).active_view().expect("a tab"))
}

/// Three pages with "alpha" on one and three, and bookmarks.
fn opened<'a>(cx: &'a mut TestAppContext, dir: &std::path::Path) -> (Entity<Viewer>, &'a mut VisualTestContext) {
    let pages: &[(f32, f32, &[&str])] = &[
        (612.0, 792.0, &["alpha one", "second line"]),
        (612.0, 792.0, &["middle page"]),
        (612.0, 792.0, &["alpha two alpha"]),
    ];
    let path = dir.join("three.pdf");
    std::fs::write(&path, pdf(pages, &[("Start", 0, &[("Inside", 1)]), ("End", 2, &[])])).expect("writes");
    windowed(cx, dir, &[path], true)
}

#[test]
fn a_pdf_opens_with_its_pages_and_bookmarks() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx) = opened(&mut cx, dir.path());
    let view = shown(cx, &viewer);
    let (sizes, marks) = cx.update(|_, app| {
        let view = view.read(app);
        (view.sizes.clone(), view.marks.iter().map(|m| (m.title.clone(), m.down.len())).collect::<Vec<_>>())
    });
    assert_eq!(sizes, [(612.0, 792.0); 3]);
    assert_eq!(marks, [("Start".to_string(), 1), ("End".to_string(), 0)]);
}

#[test]
fn find_steps_through_every_hit_across_pages() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx) = opened(&mut cx, dir.path());
    let view = shown(cx, &viewer);
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.open_find(window, cx)));
    cx.simulate_keystrokes("a l p h a");
    settle(cx, |app| view.read(app).found.len() == 3 && viewer.read(app).find.as_ref().is_some_and(|find| find.done == 3));
    let at = |cx: &mut VisualTestContext| cx.update(|_, app| {
        let view = view.read(app);
        (view.chosen, view.found[view.chosen.expect("chosen")].0)
    });
    assert_eq!(at(cx), (Some(0), 0), "the first hit is chosen");
    cx.simulate_keystrokes("enter");
    cx.simulate_keystrokes("enter");
    assert_eq!(at(cx), (Some(2), 2), "on to page three");
    cx.simulate_keystrokes("escape");
    assert!(cx.update(|_, app| viewer.read(app).find.is_none()), "escape closes it");
    assert!(cx.update(|_, app| view.read(app).found.is_empty()), "its marks go");
}

#[test]
fn a_selection_across_pages_copies_its_lines() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx) = opened(&mut cx, dir.path());
    let view = shown(cx, &viewer);
    // Ends at real characters: page two's eighth, page one's seventh.
    cx.update(|_, app| view.update(app, |view, cx| {
        view.page_text(0, cx);
        view.page_text(1, cx);
    }));
    settle(cx, |app| view.read(app).text.len() == 2);
    let centre = |cx: &mut VisualTestContext, page: usize, at: usize| cx.update(|_, app| {
        let r = view.read(app).text[&page][at].rect;
        (page, ((r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0))
    });
    let (anchor, head) = (centre(cx, 1, 7), centre(cx, 0, 6));
    cx.update(|_, app| view.update(app, |view, cx| {
        view.selection = Some(Selection { anchor, head });
        view.copy(cx);
    }));
    settle(cx, |app| app.read_from_clipboard().is_some());
    let text = cx.update(|_, app| app.read_from_clipboard().and_then(|item| item.text()));
    assert_eq!(text.as_deref(), Some("one\nsecond line\nmiddle p"), "both ends included");
}

#[test]
fn a_view_change_keeps_the_page_being_read() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx) = opened(&mut cx, dir.path());
    let view = shown(cx, &viewer);
    cx.update(|_, app| view.update(app, |view, cx| view.show(2, 0.0, cx)));
    settle(cx, |app| view.read(app).current == 2);
    cx.update(|_, app| view.update(app, |view, cx| view.change(|view, _| view.mode = Mode::Single, cx)));
    let pages = cx.update(|_, app| view.read(app).laid_out().placed.iter().map(|p| p.page).collect::<Vec<_>>());
    assert_eq!(pages, [2], "single page shows the page read");
}

#[test]
fn a_file_with_no_pages_is_refused_and_left_closed() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[], true);
    let path = file(dir.path(), "empty.pdf", &[]);
    let worker = cx.update(|_, app| viewer.read(app).core.worker.clone());
    let failed = <crate::engine::Pdf as ely_office::Engine>::load(
        &mut worker.lock().expect("the lock"),
        ely_office::Load::File(&path, ""),
    );
    assert_eq!(failed.err().map(|error| error.kind()), Some(std::io::ErrorKind::InvalidData));
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.open_file(path.clone(), cx)));
    settle(cx, |app| viewer.read(app).core.loading.is_empty());
    assert!(cx.update(|_, app| viewer.read(app).active_tab().is_none()));
}
