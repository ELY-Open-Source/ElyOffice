//! The wheel and arrows turn pages, single page too (#114).

use gpui::{Entity, ScrollDelta, ScrollEvent, TestAppContext, TouchPhase, VisualTestContext, point, px};

use super::{file, frames, settle, shown, windowed};
use crate::layout::{Mode, Zoom};
use crate::view::PdfView;

fn current(cx: &mut VisualTestContext, view: &Entity<PdfView>) -> usize {
    cx.update(|_, app| view.read(app).current)
}

fn offset(cx: &mut VisualTestContext, view: &Entity<PdfView>) -> (f32, f32) {
    let offset = cx.update(|_, app| view.read(app).scroll.offset());
    (-f32::from(offset.x), -f32::from(offset.y))
}

/// The scroll handle rounds below a thousandth.
fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

/// The lowest scroll: the page's end.
fn end(cx: &mut VisualTestContext, view: &Entity<PdfView>) -> f32 {
    cx.update(|_, app| {
        let view = view.read(app);
        (view.laid_out().size.1 - f32::from(view.room.size.height)).max(0.0)
    })
}

fn set(cx: &mut VisualTestContext, view: &Entity<PdfView>, mode: Mode, zoom: Zoom) {
    cx.update(|_, app| view.update(app, |view, cx| view.change(|view, _| (view.mode, view.zoom) = (mode, zoom), cx)));
    frames(cx);
}

fn opened<'a>(cx: &'a mut TestAppContext, dir: &std::path::Path) -> (Entity<PdfView>, &'a mut VisualTestContext) {
    let page: (f32, f32, &[&str]) = (612.0, 792.0, &["page"]);
    let (viewer, cx) = windowed(cx, dir, &[file(dir, "flip.pdf", &[page; 6])], true);
    cx.simulate_resize(gpui::size(px(1000.0), px(800.0)));
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    (view, cx)
}

#[test]
fn a_single_page_turns_when_the_wheel_goes_past_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (view, cx) = opened(&mut cx, dir.path());
    set(cx, &view, Mode::Single, Zoom::Page);
    let at = cx.update(|_, app| view.read(app).room.center());
    let wheel = |delta, touch_phase| ScrollEvent { position: at, delta, touch_phase, ..Default::default() };
    let lines = |x, y| wheel(ScrollDelta::Lines(point(x, y)), TouchPhase::Moved);
    // Down, up, then across as Shift turns the wheel.
    for (event, expected) in [(lines(0.0, -3.0), 1), (lines(0.0, -3.0), 2), (lines(0.0, 3.0), 1), (lines(-3.0, 0.0), 2), (lines(3.0, 0.0), 1)] {
        cx.simulate_event(event);
        frames(cx);
        assert_eq!(current(cx, &view), expected, "a notch past a whole page turns it");
    }
    // A trackpad swipe and its momentum turn one page.
    let pixels = |y, phase| wheel(ScrollDelta::Pixels(point(px(0.0), px(y))), phase);
    // A new touch, or a pause, starts the next swipe.
    for (swipe, touched) in [(2, true), (3, false)] {
        match touched {
            true => cx.simulate_event(pixels(-30.0, TouchPhase::Started)),
            false => cx.executor().advance_clock(std::time::Duration::from_millis(200)),
        }
        for _ in 0..30 {
            cx.simulate_event(pixels(-30.0, TouchPhase::Moved));
        }
        frames(cx);
        assert_eq!(current(cx, &view), swipe, "one page a swipe");
    }
    // A page taller than the view scrolls before it turns.
    set(cx, &view, Mode::Single, Zoom::Width);
    let page_end = end(cx, &view);
    assert!(page_end > 100.0, "the page outgrows the view: {page_end}");
    let mut notches = 0;
    while current(cx, &view) == 3 && notches < 100 {
        cx.simulate_event(lines(0.0, -3.0));
        frames(cx);
        notches += 1;
    }
    assert!(notches > 1, "the page scrolled first");
    assert_eq!(current(cx, &view), 4);
    assert!(near(offset(cx, &view).1, 0.0), "the next page from its top");
    cx.simulate_event(lines(0.0, 3.0));
    frames(cx);
    assert_eq!(current(cx, &view), 3, "back past the top");
    assert!(near(offset(cx, &view).1, end(cx, &view)), "the page before, from its end");
}

#[test]
fn arrows_scroll_and_turn_in_every_view() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (view, cx) = opened(&mut cx, dir.path());
    let focus = cx.update(|_, app| view.read(app).focus.clone());
    cx.update(|window, app| window.focus(&focus, app));
    set(cx, &view, Mode::Single, Zoom::Page);
    for (key, expected) in [("down", 1), ("right", 2), ("left", 1), ("up", 0)] {
        cx.simulate_keystrokes(key);
        frames(cx);
        assert_eq!(current(cx, &view), expected, "{key} turns a whole page");
    }
    // A taller page: arrows scroll, then turn at the edge.
    set(cx, &view, Mode::Single, Zoom::Width);
    cx.simulate_keystrokes("down");
    frames(cx);
    assert_eq!(current(cx, &view), 0);
    assert!(near(offset(cx, &view).1, 40.0), "a step down the page");
    cx.simulate_keystrokes("up up");
    frames(cx);
    assert_eq!(current(cx, &view), 0);
    assert!(near(offset(cx, &view).1, 0.0), "the first page's top stays");
    set(cx, &view, Mode::Continuous, Zoom::Page);
    for (key, expected) in [("right", 1), ("right", 2), ("left", 1)] {
        cx.simulate_keystrokes(key);
        frames(cx);
        assert_eq!(current(cx, &view), expected, "{key} turns in a column");
    }
    let before = offset(cx, &view);
    cx.simulate_keystrokes("down");
    frames(cx);
    let after = offset(cx, &view);
    assert!(near(after.0, before.0) && near(after.1, before.1 + 40.0), "down scrolls: {before:?} to {after:?}");
    // Zoomed past the width, across scrolls first.
    set(cx, &view, Mode::Continuous, Zoom::Scale(4.0));
    let (page, before) = (current(cx, &view), offset(cx, &view));
    cx.simulate_keystrokes("right");
    frames(cx);
    assert_eq!(current(cx, &view), page, "right keeps the wide page");
    assert!(near(offset(cx, &view).0, before.0 + 40.0), "right scrolls across: {before:?}");
}
