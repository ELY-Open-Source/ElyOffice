//! Reading position through the tab lifecycle.

use ely_office::{After, Closing as _, Opening as _};
use gpui::{TestAppContext, px};

use super::{file, frames, settle, shown, windowed};

#[test]
fn closing_and_reopening_restores_each_pdf_and_clamps_shorter_files() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let pages: &[(f32, f32, &[&str])] = &[
        (612.0, 792.0, &["one"]),
        (612.0, 792.0, &["two"]),
        (612.0, 792.0, &["three"]),
    ];
    let a = file(dir.path(), "a.pdf", pages);
    let b = file(dir.path(), "b.pdf", pages);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[a.clone()], true);
    for (path, page) in [(&a, 2), (&b, 1)] {
        if path == &b {
            cx.update(|_, app| viewer.update(app, |v, cx| v.open_file(path.clone(), cx)));
            settle(cx, |app| viewer.read(app).core.tabs.len() == 1);
        }
        let view = shown(cx, &viewer);
        settle(cx, |app| view.read(app).room.size.width > px(100.0));
        cx.update(|_, app| view.update(app, |v, cx| v.show(page, 0.0, cx)));
        frames(cx);
        cx.update(|window, app| {
            viewer.update(app, |v, cx| {
                v.request_close(v.core.active().expect("a tab"), After::Tab, window, cx);
            })
        });
        settle(cx, |app| viewer.read(app).core.tabs.is_empty());
    }
    for (path, page) in [(&a, 2), (&b, 1)] {
        cx.update(|_, app| viewer.update(app, |v, cx| v.open_file(path.clone(), cx)));
        settle(cx, |app| viewer.read(app).core.tabs.len() == 1);
        let view = shown(cx, &viewer);
        settle(cx, |app| {
            view.read(app).restore_page.is_none() && view.read(app).current == page
        });
        assert!(cx.update(|_, app| view.read(app).scroll.offset().y < px(0.0)));
        cx.update(|window, app| {
            viewer.update(app, |v, cx| {
                v.request_close(v.core.active().expect("a tab"), After::Tab, window, cx);
            })
        });
        settle(cx, |app| viewer.read(app).core.tabs.is_empty());
    }
    file(dir.path(), "a.pdf", &pages[..1]);
    cx.update(|_, app| viewer.update(app, |v, cx| v.open_file(a, cx)));
    settle(cx, |app| viewer.read(app).core.tabs.len() == 1);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).restore_page.is_none());
    assert_eq!(cx.update(|_, app| view.read(app).current), 0);
}

#[test]
fn manual_navigation_cancels_a_pending_restore() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let pages: &[(f32, f32, &[&str])] = &[
        (612.0, 792.0, &["one"]), (612.0, 792.0, &["two"]),
    ];
    let path = file(dir.path(), "manual.pdf", pages);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    cx.update(|_, app| view.update(app, |view, cx| {
        view.restore_page = Some(1);
        view.show(0, 0.0, cx);
        assert_eq!(view.restore_page, None);
    }));
    frames(cx);
    assert_eq!(cx.update(|_, app| view.read(app).current), 0);
}
