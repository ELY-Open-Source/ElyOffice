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
            view.read(app).restore_view.is_none() && view.read(app).current == page
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
    settle(cx, |app| view.read(app).restore_view.is_none());
    assert_eq!(cx.update(|_, app| view.read(app).current), 0);
}

#[test]
fn manual_navigation_cancels_a_pending_restore() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let pages: &[(f32, f32, &[&str])] = &[(612.0, 792.0, &["one"]), (612.0, 792.0, &["two"])];
    let path = file(dir.path(), "manual.pdf", pages);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    cx.update(|_, app| {
        view.update(app, |view, cx| {
            view.restore_view = Some(crate::history::Reading::at_page(1));
            view.show(0, 0.0, cx);
            assert_eq!(view.restore_view, None);
        })
    });
    frames(cx);
    assert_eq!(cx.update(|_, app| view.read(app).current), 0);
}

#[test]
fn reopening_restores_zoom_mode_rotation_scroll_and_sidebar() {
    use crate::{
        layout::{Mode, Zoom},
        side::Pane,
    };
    use gpui::point;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let page: (f32, f32, &[&str]) = (612.0, 792.0, &["page"]);
    let a = file(dir.path(), "full-view.pdf", &[page; 8]);
    let b = file(dir.path(), "other.pdf", &[page; 8]);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[a.clone()], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    cx.update(|_, app| {
        viewer.update(app, |v, cx| {
            v.side.update(cx, |side, cx| {
                side.pane = Pane::Notes;
                cx.notify();
            });
            ely_ui::settings::panels::show(true, false, cx);
        })
    });
    frames(cx);
    cx.update(|_, app| {
        view.update(app, |v, cx| {
            (v.mode, v.zoom, v.rotate) = (Mode::Horizontal, Zoom::Scale(2.0), 90);
            v.show(2, 0.0, cx);
            let placed = v.laid_out().placed[2];
            v.go_to = Some((placed.x + 80.0, 120.0));
            cx.notify();
        })
    });
    frames(cx);
    let before = cx.update(|_, app| view.read(app).scroll.offset());
    assert!(before.x < px(-100.0) && before.y < px(-100.0));
    cx.update(|window, app| {
        viewer.update(app, |v, cx| {
            v.request_close(v.core.active().expect("a tab"), After::Tab, window, cx);
        })
    });
    settle(cx, |app| viewer.read(app).core.tabs.is_empty());
    cx.update(|_, app| viewer.update(app, |v, cx| v.open_file(b, cx)));
    settle(cx, |app| viewer.read(app).core.tabs.len() == 1);
    cx.update(|_, app| {
        viewer.update(app, |v, cx| {
            v.side.update(cx, |side, cx| {
                side.pane = Pane::Pages;
                cx.notify();
            });
            ely_ui::settings::panels::show(true, true, cx);
        })
    });
    frames(cx);
    cx.update(|window, app| {
        viewer.update(app, |v, cx| {
            v.request_close(v.core.active().expect("a tab"), After::Tab, window, cx);
        })
    });
    settle(cx, |app| viewer.read(app).core.tabs.is_empty());
    cx.update(|_, app| viewer.update(app, |v, cx| v.open_file(a, cx)));
    settle(cx, |app| viewer.read(app).core.tabs.len() == 1);
    let restored = shown(cx, &viewer);
    settle(cx, |app| restored.read(app).restore_view.is_none());
    frames(cx);
    cx.update(|_, app| {
        let v = restored.read(app);
        assert_eq!(
            (v.mode, v.zoom, v.rotate),
            (Mode::Horizontal, Zoom::Scale(2.0), 90)
        );
        assert_eq!(v.current, 2);
        let after = v.scroll.offset();
        assert!(
            (after.x - before.x).abs() < px(1.0),
            "horizontal position: {before:?} -> {after:?}"
        );
        assert!(
            (after.y - before.y).abs() < px(1.0),
            "vertical position: {before:?} -> {after:?}"
        );
        assert_eq!(viewer.read(app).side.read(app).pane, Pane::Notes);
        assert!(!ely_ui::settings::panels::shown(true, app));
        assert_ne!(after, point(px(0.0), px(0.0)));
    });
}

#[test]
fn fit_width_keeps_the_saved_spot_while_the_sidebar_animates() {
    for page in [5, 11] {
        assert_fit_width_restore(page);
    }
}

fn assert_fit_width_restore(page_index: usize) {
    use crate::layout::Zoom;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let page: (f32, f32, &[&str]) = (612.0, 792.0, &["page"]);
    let path = file(dir.path(), "fit-width.pdf", &[page; 12]);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path.clone()], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    cx.update(|_, app| ely_ui::settings::panels::show(true, false, app));
    let full_width = cx.update(|window, _| window.viewport_size().width);
    settle(cx, |app| {
        view.read(app).room.size.width > full_width - px(4.0)
    });
    let expected_width = cx.update(|_, app| view.read(app).room.size.width);
    cx.update(|_, app| {
        view.update(app, |v, cx| {
            v.zoom = Zoom::Width;
            v.show(page_index, 200.0, cx);
            if page_index == 11 {
                let laid = v.laid_out();
                v.go_to = Some((0.0, (laid.size.1 - f32::from(v.room.size.height)).max(0.0)));
            }
        })
    });
    frames(cx);
    let expected_offset = cx.update(|_, app| view.read(app).scroll.offset());
    cx.update(|window, app| {
        viewer.update(app, |v, cx| {
            v.request_close(v.core.active().expect("a tab"), After::Tab, window, cx);
        })
    });
    settle(cx, |app| viewer.read(app).core.tabs.is_empty());
    cx.update(|_, app| ely_ui::settings::panels::show(true, true, app));
    frames(cx);
    cx.update(|_, app| viewer.update(app, |v, cx| v.open_file(path, cx)));
    settle(cx, |app| viewer.read(app).core.tabs.len() == 1);
    let restored = shown(cx, &viewer);
    settle(cx, |app| {
        let v = restored.read(app);
        v.restore_view.is_none() && (v.room.size.width - expected_width).abs() < px(0.5)
    });
    frames(cx);
    cx.update(|_, app| {
        let v = restored.read(app);
        assert_eq!(v.current, page_index);
        assert_eq!(v.zoom, Zoom::Width);
        assert!(
            (v.scroll.offset().y - expected_offset.y).abs() < px(1.0),
            "{expected_offset:?} -> {:?}",
            v.scroll.offset()
        );
    });
}

#[test]
fn page_edits_discard_the_previous_layout_anchor() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let mut pages: Vec<(f32, f32, &[&str])> = vec![(612.0, 792.0, &["page"]); 8];
    pages[0] = (612.0, 5000.0, &["tall page"]);
    let path = file(dir.path(), "edited.pdf", &pages);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    cx.update(|_, app| {
        view.update(app, |v, cx| {
            v.restore_reading(crate::history::Reading::at_page(5), cx)
        })
    });
    frames(cx);
    assert!(cx.update(|_, app| view.read(app).layout_anchor.is_some()));
    cx.update(|_, app| {
        view.update(app, |v, cx| {
            v.edit_pages(
                "pdf.doing.delete_pages",
                |worker, doc| worker.delete_pages(doc, &[0]),
                cx,
            );
        })
    });
    settle(cx, |app| view.read(app).settled());
    frames(cx);
    assert!(cx.update(|_, app| view.read(app).layout_anchor.is_none()));
    let before = cx.update(|_, app| view.read(app).current);
    let viewport = cx.update(|window, _| window.viewport_size());
    cx.simulate_resize(gpui::size(viewport.width + px(100.0), viewport.height));
    frames(cx);
    assert_eq!(cx.update(|_, app| view.read(app).current), before);
}
