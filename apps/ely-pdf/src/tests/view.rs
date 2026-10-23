//! Password prompts, zoomed pairs, jumps across (C11.1).

use ely_office::{Dialog, Dialogs as _, Opening as _, Tabs as _};
use gpui::{Entity, Modifiers, ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext, point, px};

use super::{file, frames, settle, shown, windowed};
use crate::layout::{Mode, Zoom};
use ely_gpui_component::shell::ZoomStep;
use crate::view::PdfView;
use crate::viewer::Viewer;

/// The file the open prompt asks for.
fn asking(app: &gpui::App, viewer: &Entity<Viewer>) -> Option<std::path::PathBuf> {
    match &viewer.read(app).core.dialog {
        Some(Dialog::Password { path, .. }) => Some(path.clone()),
        _ => None,
    }
}

/// Two files asked for at once, as locked loads do.
fn asked<'a>(cx: &'a mut TestAppContext, dir: &std::path::Path) -> (Entity<Viewer>, &'a mut VisualTestContext, [std::path::PathBuf; 2]) {
    let (viewer, cx) = windowed(cx, dir, &[], true);
    let paths = [file(dir, "a.pdf", &[(612.0, 792.0, &["a"])]), file(dir, "b.pdf", &[(612.0, 792.0, &["b"])])];
    cx.update(|window, app| viewer.update(app, |viewer, cx| {
        for path in &paths {
            viewer.ask_password(path.clone(), false, false, window, cx);
        }
    }));
    frames(cx);
    (viewer, cx, paths)
}

#[test]
fn a_typed_password_opens_the_file() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, paths) = asked(&mut cx, dir.path());
    // An open file takes any password.
    cx.simulate_input("secret");
    cx.simulate_keystrokes("enter");
    settle(cx, |app| viewer.read(app).core.tabs.len() == 1);
    assert_eq!(cx.update(|_, app| asking(app, &viewer)), Some(paths[1].clone()), "the next asks");
}

#[test]
fn escape_gives_up_each_queued_file_in_turn() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, paths) = asked(&mut cx, dir.path());
    assert_eq!(cx.update(|_, app| asking(app, &viewer)), Some(paths[0].clone()));
    cx.simulate_keystrokes("escape");
    settle(cx, |app| asking(app, &viewer) == Some(paths[1].clone()));
    frames(cx);
    cx.simulate_keystrokes("escape");
    settle(cx, |app| viewer.read(app).core.dialog.is_none());
    assert!(cx.update(|_, app| viewer.read(app).active_tab().is_none()));
}

/// Two zoomed Letter pages side by side.
fn zoomed_pair<'a>(cx: &'a mut TestAppContext, dir: &std::path::Path) -> (Entity<PdfView>, &'a mut VisualTestContext) {
    let pages: &[(f32, f32, &[&str])] = &[(612.0, 792.0, &["one"]), (612.0, 792.0, &["two"])];
    let (viewer, cx) = windowed(cx, dir, &[file(dir, "pair.pdf", pages)], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    cx.update(|_, app| view.update(app, |view, cx| {
        (view.mode, view.zoom) = (Mode::Two, Zoom::Scale(4.0));
        cx.notify();
    }));
    frames(cx);
    (view, cx)
}

#[test]
fn two_pages_keep_the_right_hand_page() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let pages: &[(f32, f32, &[&str])] = &[(612.0, 792.0, &["one"]), (612.0, 792.0, &["two"]), (612.0, 792.0, &["three"])];
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[file(dir.path(), "three.pdf", pages)], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    cx.update(|_, app| view.update(app, |view, cx| {
        view.mode = Mode::Two;
        view.show(1, 0.0, cx);
    }));
    frames(cx);
    assert_eq!(cx.update(|_, app| view.read(app).current), 1);
}

#[test]
fn scrolling_across_to_the_right_page_makes_it_current() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (view, cx) = zoomed_pair(&mut cx, dir.path());
    cx.update(|_, app| view.update(app, |view, cx| {
        let right = view.laid_out().placed[1].x;
        let y = view.scroll.offset().y;
        view.scroll.set_offset(point(px(-right), y));
        cx.notify();
    }));
    frames(cx);
    assert_eq!(cx.update(|_, app| view.read(app).current), 1);
}

#[test]
fn a_jump_to_the_right_page_brings_it_across() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (view, cx) = zoomed_pair(&mut cx, dir.path());
    cx.update(|_, app| view.update(app, |view, cx| view.show(1, 0.0, cx)));
    frames(cx);
    let (x, left, wide) = cx.update(|_, app| {
        let view = view.read(app);
        (view.laid_out().placed[1].x, -f32::from(view.scroll.offset().x), f32::from(view.room.size.width))
    });
    assert!(x >= left && x < left + wide, "page at {x}, view {left}..{}", left + wide);
}

#[test]
fn ctrl_and_a_wheel_notch_step_the_zoom_once() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let pages: &[(f32, f32, &[&str])] = &[(612.0, 792.0, &["one"]), (612.0, 792.0, &["two"]), (612.0, 792.0, &["three"])];
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[file(dir.path(), "three.pdf", pages)], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    cx.update(|_, app| view.update(app, |view, cx| view.change(|view, _| view.zoom = Zoom::Scale(1.0), cx)));
    frames(cx);
    let at = cx.update(|_, app| view.read(app).room.center());
    let wheel = |modifiers, y| ScrollWheelEvent { position: at, delta: ScrollDelta::Lines(point(0.0, y)), modifiers, ..Default::default() };
    let other = match cfg!(target_os = "macos") {
        true => Modifiers { control: true, ..Default::default() },
        false => Modifiers { platform: true, ..Default::default() },
    };
    let read = |cx: &mut VisualTestContext| cx.update(|_, app| (view.read(app).zoom, view.read(app).scroll.offset().y));
    cx.simulate_event(wheel(other, -3.0));
    frames(cx);
    let (zoom, scrolled) = read(cx);
    assert_eq!(zoom, Zoom::Scale(1.0), "only Ctrl, or Cmd on macOS");
    assert!(scrolled < px(0.0), "a plain wheel scrolls");
    cx.simulate_event(wheel(Modifiers::secondary_key(), 3.0));
    frames(cx);
    assert_eq!(read(cx).0, Zoom::Scale(crate::keys::stepped(1.0, true)), "a notch is one step");
    cx.simulate_event(wheel(Modifiers::secondary_key(), -3.0));
    cx.simulate_event(wheel(Modifiers::secondary_key(), -3.0));
    frames(cx);
    assert_eq!(read(cx).0, Zoom::Scale(crate::keys::stepped(1.0, false)));
}

#[test]
fn a_dark_app_shows_the_page_as_the_file_paints_it() {
    use ely_gpui_component::theme::{Mode, Theme};
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[], true);
    cx.update(|_, app| Theme::set_mode_now(Mode::Dark, app));
    let path = file(dir.path(), "one.pdf", &[(612.0, 792.0, &["alpha"])]);
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.open_file(path, cx)));
    settle(cx, |app| viewer.read(app).active_view().is_some());
    let view = shown(cx, &viewer);
    let corner = |app: &gpui::App| view.read(app).tiles.iter().find(|(key, _)| key.tx == 0 && key.ty == 0).map(|(_, image)| image.clone());
    settle(cx, |app| corner(app).is_some());
    let tile = cx.update(|_, app| corner(app)).expect("the first tile");
    let bytes = tile.as_bytes(0).expect("its pixels");
    assert_eq!(&bytes[..3], &[255, 255, 255], "the paper stays white");
}

#[test]
fn a_default_zoom_opens_each_file_at_it() {
    use ely_platform::settings::DocZoom;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[], true);
    let page: &[(f32, f32, &[&str])] = &[(612.0, 792.0, &["one"])];
    let zooms = [(DocZoom::Auto, Zoom::Width), (DocZoom::Percent(150), Zoom::Scale(1.5 * crate::layout::ACTUAL))];
    for (n, (chosen, opened)) in zooms.into_iter().enumerate() {
        cx.update(|_, app| app.global_mut::<ely_ui::settings::Current>().shared.doc_zoom = chosen);
        let path = file(dir.path(), &format!("{n}.pdf"), page);
        cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.open_file(path, cx)));
        settle(cx, |app| viewer.read(app).core.tabs.len() == n + 1);
        let view = shown(cx, &viewer);
        assert_eq!(cx.update(|_, app| view.read(app).zoom), opened, "{chosen:?}");
    }
    let view = shown(cx, &viewer);
    assert_eq!(cx.update(|_, app| view.read(app).percent()), 150, "150% shows as 150");
}

/// The units digit the zoom control draws.
fn units(cx: &mut VisualTestContext) -> usize {
    const DIGITS: [&str; 10] = ["rolling-0", "rolling-1", "rolling-2", "rolling-3", "rolling-4", "rolling-5", "rolling-6", "rolling-7", "rolling-8", "rolling-9"];
    std::thread::sleep(std::time::Duration::from_millis(5));
    frames(cx);
    // The last cell drawn holds the units.
    let cell = cx.debug_bounds("rolling-cell").expect("the control's digits");
    (0..10)
        .find(|&d| cx.debug_bounds(DIGITS[d]).is_some_and(|b| (b.top() - cell.top()).abs() < px(0.5)))
        .expect("one digit in view")
}

#[test]
fn the_status_bar_steps_the_zoom() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx) = super::opened(&mut cx, dir.path());
    cx.update(|_, app| ely_gpui_component::theme::Theme::update(app, |theme| theme.reduced_motion = true));
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    let scale = |cx: &mut VisualTestContext| cx.update(|_, app| view.read(app).laid_out().scale);
    let press = |cx: &mut VisualTestContext, x: fn(gpui::Bounds<gpui::Pixels>) -> gpui::Pixels| {
        let bar = cx.debug_bounds("pdf-zoom").expect("the zoom control");
        cx.simulate_click(point(x(bar), bar.center().y), Modifiers::none());
        frames(cx);
    };
    let start = scale(cx);
    // Minus left, percent between, plus right.
    press(cx, |bar| bar.left() + px(8.0));
    assert_eq!(scale(cx), crate::keys::stepped(start, false), "minus steps down");
    press(cx, |bar| bar.right() - px(8.0));
    assert_eq!(scale(cx), crate::keys::stepped(crate::keys::stepped(start, false), true), "plus steps up");
    press(cx, |bar| bar.center().x);
    assert_eq!(cx.update(|_, app| view.read(app).zoom), Zoom::Scale(crate::layout::ACTUAL), "the percent is actual size");
    assert_eq!(units(cx), 0, "actual size draws 100");
    cx.update(|_, app| view.update(app, |view, cx| view.change(|view, _| view.zoom = Zoom::Scale(1.25 * crate::layout::ACTUAL), cx)));
    assert_eq!(units(cx), 5, "125% draws 125");
}

#[test]
fn a_zoom_keeps_the_spot_on_the_page_at_the_top() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let page: (f32, f32, &[&str]) = (612.0, 792.0, &["p"]);
    let path = file(dir.path(), "ten.pdf", &[page; 10]);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    let at = cx.update(|_, app| view.read(app).room.center());
    for _ in 0..8 {
        cx.simulate_event(ScrollWheelEvent { position: at, delta: ScrollDelta::Pixels(point(px(0.0), px(-500.0))), ..Default::default() });
        frames(cx);
    }
    let before = spot(cx, &view);
    assert!(before.0 >= 1 && before.2 > 50.0, "partway into a later page: {before:?}");
    for step in [ZoomStep::In, ZoomStep::In, ZoomStep::Out] {
        cx.update(|_, app| viewer.update(app, |viewer, cx| crate::keys::zoom(viewer, step, cx)));
        frames(cx);
        let after = spot(cx, &view);
        assert!(same(after, before), "{step:?} keeps the spot: {before:?} -> {after:?}");
    }
    // Two steps before a frame, as one fast wheel.
    cx.update(|_, app| {
        for _ in 0..2 {
            viewer.update(app, |viewer, cx| crate::keys::zoom(viewer, ZoomStep::Out, cx));
        }
    });
    frames(cx);
    let after = spot(cx, &view);
    assert!(same(after, before), "two quick steps keep the spot: {before:?} -> {after:?}");
}

/// The page under the top edge, and its spot there.
fn spot(cx: &mut VisualTestContext, view: &Entity<PdfView>) -> (usize, f32, f32, f32) {
    cx.update(|_, app| {
        let view = view.read(app);
        let (laid, offset) = (view.laid_out(), view.scroll.offset());
        let (left, top) = (-f32::from(offset.x), -f32::from(offset.y));
        let below = || laid.placed.iter().filter(|p| p.y + p.h > top);
        let column = below().filter(|p| p.x <= left && p.x + p.w > left).min_by(|a, b| a.y.total_cmp(&b.y));
        let place = column.or_else(|| below().next()).expect("a page at the top");
        (place.page, (left - place.x) / laid.scale, (top - place.y) / laid.scale, laid.scale)
    })
}

/// Same spot; across only when panned in, above in pixels.
fn same(a: (usize, f32, f32, f32), b: (usize, f32, f32, f32)) -> bool {
    let down = match b.2 < 0.0 {
        true => (a.2 * a.3 - b.2 * b.3).abs() < 0.5,
        false => (a.2 - b.2).abs() < 0.5,
    };
    a.0 == b.0 && (b.1 < 0.0 || (a.1 - b.1).abs() < 0.5) && down
}

/// `pages` at scale 2, held at `at` on `page`.
fn held<'a>(
    cx: &'a mut TestAppContext,
    dir: &std::path::Path,
    pages: &[(f32, f32, &[&str])],
    (mode, page): (Mode, usize),
    at: (f32, f32),
) -> (Entity<Viewer>, &'a mut VisualTestContext, Entity<PdfView>) {
    let (viewer, cx) = windowed(cx, dir, &[file(dir, "held.pdf", pages)], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > px(100.0));
    cx.update(|_, app| view.update(app, |view, cx| view.change(|view, _| (view.zoom, view.mode) = (Zoom::Scale(2.0), mode), cx)));
    frames(cx);
    cx.update(|_, app| view.update(app, |view, cx| {
        let place = *view.laid_out().placed.iter().find(|p| p.page == page).expect("the page");
        view.go_to = Some((place.x + at.0 * 2.0, place.y + at.1 * 2.0));
        cx.notify();
    }));
    frames(cx);
    (viewer, cx, view)
}

#[test]
fn zooming_out_near_a_page_bottom_keeps_the_spot() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = held(&mut cx, dir.path(), &[(612.0, 792.0, &["p"] as &[&str]); 10], (Mode::Continuous, 1), (0.0, 764.5));
    let before = spot(cx, &view);
    assert_eq!((before.0, before.2), (1, 764.5), "near page 2's bottom");
    for _ in 0..2 {
        cx.update(|_, app| viewer.update(app, |viewer, cx| crate::keys::zoom(viewer, ZoomStep::Out, cx)));
        frames(cx);
        let after = spot(cx, &view);
        assert!(same(after, before), "a step out keeps the spot: {before:?} -> {after:?}");
    }
}

#[test]
fn a_zoom_keeps_the_spot_across() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = held(&mut cx, dir.path(), &[(1200.0, 792.0, &["p"] as &[&str]); 10], (Mode::Continuous, 1), (200.0, 284.0));
    let before = spot(cx, &view);
    assert!(before.1 > 150.0, "panned into the page: {before:?}");
    cx.update(|_, app| viewer.update(app, |viewer, cx| crate::keys::zoom(viewer, ZoomStep::In, cx)));
    frames(cx);
    let after = spot(cx, &view);
    assert!(same(after, before), "a step in keeps the spot across: {before:?} -> {after:?}");
}

#[test]
fn a_zoom_on_the_right_hand_page_keeps_its_spot() {
    // Wide enough to pan into the right-hand page.
    let short: (f32, f32, &[&str]) = (1000.0, 200.0, &["p"]);
    let letter: (f32, f32, &[&str]) = (1000.0, 792.0, &["p"]);
    for pages in [[letter; 10], [short, letter, short, letter, short, letter, short, letter, short, letter]] {
        // On the page, then in the gap above its row.
        for at in [(100.0, 50.0), (100.0, -4.0)] {
            let dir = tempfile::tempdir().expect("a temp dir");
            let mut cx = TestAppContext::single();
            let (viewer, cx, view) = held(&mut cx, dir.path(), &pages, (Mode::Two, 3), at);
            let before = spot(cx, &view);
            assert_eq!(before.0, 3, "by the right-hand page: {before:?}");
            cx.update(|_, app| viewer.update(app, |viewer, cx| crate::keys::zoom(viewer, ZoomStep::In, cx)));
            frames(cx);
            let after = spot(cx, &view);
            assert!(same(after, before), "a step in at {at:?} keeps the spot: {before:?} -> {after:?}");
        }
    }
}
