//! The page bar drags the view (#84).

use gpui::{Bounds, Modifiers, MouseButton, TestAppContext, point, px, size};

use super::{file, frames, settle, shown, windowed};

#[test]
fn a_dragged_bar_moves_the_pages_and_a_resize_keeps_them() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let page: (f32, f32, &[&str]) = (612.0, 792.0, &["page"]);
    let pages = vec![page; 30];
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[file(dir.path(), "thirty.pdf", &pages)], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    let room = cx.update(|_, app| view.read(app).room);
    // A resize before any move keeps a place.
    cx.update(|_, app| view.update(app, |view, cx| view.resize_reading(room, cx)));
    frames(cx);
    let thumb = cx.debug_bounds("scrollbar-thumb").expect("the page bar");
    let grip = thumb.center();
    cx.simulate_mouse_move(grip, None, Modifiers::none());
    cx.simulate_mouse_down(grip, MouseButton::Left, Modifiers::none());
    let down = room.size.height / 2.0;
    for step in [px(10.0), px(20.0), down] {
        cx.simulate_mouse_move(grip + point(px(0.0), step), MouseButton::Left, Modifiers::none());
    }
    cx.simulate_mouse_up(grip + point(px(0.0), down), MouseButton::Left, Modifiers::none());
    frames(cx);
    let (dragged, current) = cx.update(|_, app| (-view.read(app).scroll.offset().y, view.read(app).current));
    assert!(dragged > px(5000.0), "half the bar is half the file: {dragged:?}");
    assert!(current > 10, "the page read follows the bar: {current}");
    let narrower = Bounds::new(room.origin, size(room.size.width - px(40.0), room.size.height));
    cx.update(|_, app| view.update(app, |view, cx| view.resize_reading(narrower, cx)));
    frames(cx);
    let kept = cx.update(|_, app| view.read(app).current);
    assert_eq!(kept, current, "a resize keeps the dragged page");
}
