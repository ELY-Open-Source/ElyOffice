//! A row of pages and which one is current (R13.1).

use gpui::{Modifiers, ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, point, px};

use super::{file, frames, settle, shown, windowed};
use crate::layout::{Mode, Zoom};

#[test]
fn shift_and_the_wheel_scroll_a_row_of_pages_across() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let pages: &[(f32, f32, &[&str])] = &[(612.0, 792.0, &["one"]), (612.0, 792.0, &["two"]), (612.0, 792.0, &["three"])];
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[file(dir.path(), "row.pdf", pages)], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    cx.update(|_, app| view.update(app, |view, cx| view.change(|view, _| (view.mode, view.zoom) = (Mode::Horizontal, Zoom::Width), cx)));
    frames(cx);
    let at = cx.update(|_, app| view.read(app).room.center());
    let offset = |cx: &mut VisualTestContext| cx.update(|_, app| view.read(app).scroll.offset());
    let second = cx.update(|_, app| view.read(app).laid_out().placed[1].x);
    // Platforms turn Shift and the wheel into an across delta.
    let notch = ScrollEvent { position: at, delta: ScrollDelta::Lines(point(-3.0, 0.0)), modifiers: Modifiers::shift(), ..Default::default() };
    for _ in 0..100 {
        if -f32::from(offset(cx).x) >= second {
            break;
        }
        cx.simulate_event(notch.clone());
        frames(cx);
    }
    assert!(-f32::from(offset(cx).x) >= second, "scrolled to {:?}", offset(cx));
    assert_eq!(offset(cx).y, px(0.0), "across only");
    assert_eq!(cx.update(|_, app| view.read(app).current), 1, "the page in view is current");
    // A sliver of the first page still shows.
    cx.update(|_, app| view.update(app, |view, cx| {
        view.scroll.set_offset(point(px(-(second - crate::layout::GAP - 20.0)), px(0.0)));
        cx.notify();
    }));
    frames(cx);
    assert_eq!(cx.update(|_, app| view.read(app).current), 1, "a sliver is not current");
    cx.update(|_, app| view.update(app, |view, cx| view.turn(1, cx)));
    frames(cx);
    let third = cx.update(|_, app| view.read(app).laid_out().placed[2].x);
    assert_eq!(cx.update(|_, app| view.read(app).current), 2);
    let left = -f32::from(offset(cx).x);
    // The scroll handle rounds below a thousandth of a pixel.
    assert!((left - (third - crate::layout::GAP)).abs() < 0.001, "the next page comes to the left: {left} vs {third}");
}

/// Twenty small pages, several in view along `mode`.
fn small_pages(mode: Mode) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let pages: Vec<(f32, f32, &[&str])> = (0..20).map(|_| (612.0, 792.0, &["p"] as &[&str])).collect();
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[file(dir.path(), "small.pdf", &pages)], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    cx.update(|_, app| view.update(app, |view, cx| view.change(|view, _| (view.mode, view.zoom) = (mode, Zoom::Scale(0.2)), cx)));
    frames(cx);
    let (all, room, page) = cx.update(|_, app| {
        let (view, laid) = (view.read(app), view.read(app).laid_out());
        match mode {
            Mode::Horizontal => (laid.size.0, f32::from(view.room.size.width), laid.placed[0].w),
            _ => (laid.size.1, f32::from(view.room.size.height), laid.placed[0].h),
        }
    });
    assert!(all > room && room > 2.0 * page, "{all} long in {room}, pages {page}");
    let current = |cx: &mut VisualTestContext| cx.update(|_, app| view.read(app).current);
    cx.update(|_, app| view.update(app, |view, cx| view.show(19, 0.0, cx)));
    frames(cx);
    assert_eq!(current(cx), 19, "a jump to the last page");
    cx.update(|_, app| view.update(app, |view, cx| view.show(0, 0.0, cx)));
    frames(cx);
    for want in 1..20 {
        cx.update(|_, app| view.update(app, |view, cx| view.turn(1, cx)));
        frames(cx);
        assert_eq!(current(cx), want, "Next from {}", want - 1);
    }
    // Off the end, the reading order rules again.
    let edge = all - room - page;
    cx.update(|_, app| view.update(app, |view, cx| {
        let at = match mode {
            Mode::Horizontal => point(px(-edge), px(0.0)),
            _ => point(px(0.0), px(-edge)),
        };
        view.scroll.set_offset(at);
        cx.notify();
    }));
    frames(cx);
    let first = cx.update(|_, app| {
        let laid = view.read(app).laid_out();
        let far = |p: &crate::layout::Placed| match mode {
            Mode::Horizontal => p.x + p.w,
            _ => p.y + p.h,
        };
        laid.placed.iter().find(|p| far(p) > edge + crate::layout::GAP * 3.0).map(|p| p.page)
    });
    assert_eq!(Some(current(cx)), first, "scrolled back from the end");
    assert!(current(cx) < 19);
}

/// A hundred `size` pages, each thinner than the sliver margin.
fn thin_pages(mode: Mode, size: (f32, f32), zoom: Zoom) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let pages: Vec<(f32, f32, &[&str])> = (0..100).map(|_| (size.0, size.1, &["p"] as &[&str])).collect();
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[file(dir.path(), "thin.pdf", &pages)], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    cx.update(|_, app| view.update(app, |view, cx| view.change(|view, _| (view.mode, view.zoom) = (mode, zoom), cx)));
    frames(cx);
    let thin = cx.update(|_, app| {
        let place = view.read(app).laid_out().placed[2];
        if mode == Mode::Horizontal { place.w } else { place.h }
    });
    // A jump leaves one gap across, two down.
    let lead = if mode == Mode::Horizontal { 1.0 } else { 2.0 };
    assert!(thin + crate::layout::GAP * lead < crate::layout::GAP * 3.0, "pages {thin} thin");
    cx.update(|_, app| view.update(app, |view, cx| view.show(2, 0.0, cx)));
    frames(cx);
    assert_eq!(cx.update(|_, app| view.read(app).current), 2);
}

#[test]
fn a_jump_to_a_thin_page_across_makes_it_current() {
    thin_pages(Mode::Horizontal, (200.0, 8000.0), Zoom::Page);
}

#[test]
fn a_jump_to_a_short_page_down_makes_it_current() {
    thin_pages(Mode::Continuous, (8000.0, 50.0), Zoom::Width);
}

#[test]
fn scrolling_onto_the_end_drops_a_page_left_as_a_sliver() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let pages: Vec<(f32, f32, &[&str])> = (0..10).map(|_| (612.0, 792.0, &["p"] as &[&str])).collect();
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[file(dir.path(), "end.pdf", &pages)], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    cx.update(|_, app| view.update(app, |view, cx| view.change(|view, _| (view.mode, view.zoom) = (Mode::Horizontal, Zoom::Scale(crate::layout::ACTUAL)), cx)));
    frames(cx);
    let end = cx.update(|_, app| view.read(app).laid_out().size.0 - f32::from(view.read(app).room.size.width));
    let at = |cx: &mut VisualTestContext, left: f32| {
        cx.update(|_, app| view.update(app, |view, cx| {
            view.scroll.set_offset(point(px(-left), px(0.0)));
            cx.notify();
        }));
        frames(cx);
        cx.update(|_, app| view.read(app).current)
    };
    let before = at(cx, end - 100.0);
    let first = cx.update(|_, app| {
        let laid = view.read(app).laid_out();
        let gap = crate::layout::GAP;
        laid.placed.iter().find(|p| p.x + p.w > end + (gap * 3.0).min(gap + p.w / 2.0)).map(|p| p.page)
    });
    assert_ne!(Some(before), first, "the page read becomes a sliver");
    assert_eq!(Some(at(cx, end)), first);
}

#[test]
fn pages_that_never_reach_the_left_still_turn_current() {
    small_pages(Mode::Horizontal);
}

#[test]
fn pages_that_never_reach_the_top_still_turn_current() {
    small_pages(Mode::Continuous);
}
