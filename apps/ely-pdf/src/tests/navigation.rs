//! Page navigation through the centered toolbar.

use super::{file, frames, settle, shown, windowed};
use crate::layout::Mode;
use gpui::{Modifiers, TestAppContext, px};

#[test]
fn a_page_jump_reports_the_new_page_in_every_view_mode() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let pages: &[(f32, f32, &[&str])] = &[
        (612.0, 792.0, &["one"]),
        (612.0, 792.0, &["two"]),
        (612.0, 792.0, &["three"]),
    ];
    let (viewer, cx) = windowed(
        &mut cx,
        dir.path(),
        &[file(dir.path(), "jump.pdf", pages)],
        true,
    );
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    let reports = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = reports.clone();
    cx.update(|_, app| {
        app.subscribe(&view, move |_, _: &crate::view::Reported, _| {
            counted.set(counted.get() + 1);
        })
        .detach();
    });
    for mode in [Mode::Continuous, Mode::Single, Mode::Two, Mode::Horizontal] {
        cx.update(|_, app| {
            view.update(app, |view, cx| {
                view.mode = mode;
                view.show(0, 0.0, cx);
            })
        });
        frames(cx);
        let before = reports.get();
        cx.update(|_, app| view.update(app, |view, cx| view.show(2, 0.0, cx)));
        frames(cx);
        assert_eq!(cx.update(|_, app| view.read(app).current), 2);
        assert!(reports.get() > before, "the page field hears the jump");
    }
}

#[test]
fn typing_a_page_waits_for_enter_and_respects_the_document_range() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let page: (f32, f32, &[&str]) = (612.0, 792.0, &["page"]);
    let path = file(dir.path(), "fifteen.pdf", &[page; 15]);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    frames(cx);
    for width in [640.0, 1100.0] {
        cx.simulate_resize(gpui::size(px(width), px(800.0)));
        frames(cx);
        let navigation = cx.debug_bounds("pdf-page-navigation").expect("page navigation");
        let viewport = cx.update(|window, _| window.viewport_size());
        assert!((navigation.center().x - viewport.width / 2.0).abs() < px(1.0));
        assert!(navigation.bottom() <= cx.update(|_, app| view.read(app).room.top()));
    }
    let input = cx.debug_bounds("pdf-page-input").expect("page field");
    cx.simulate_click(input.center(), Modifiers::none());
    cx.simulate_keystrokes("secondary-a");
    cx.simulate_input("12");
    frames(cx);
    assert_eq!(
        cx.update(|_, app| view.read(app).current),
        0,
        "typing does not jump"
    );
    cx.simulate_keystrokes("enter");
    frames(cx);
    assert_eq!(
        cx.update(|_, app| view.read(app).current),
        11,
        "one-based input"
    );
    for (typed, expected) in [("999", 14), ("0", 0)] {
        cx.simulate_keystrokes("secondary-a");
        cx.simulate_input(typed);
        cx.simulate_keystrokes("enter");
        frames(cx);
        assert_eq!(cx.update(|_, app| view.read(app).current), expected);
    }
}
