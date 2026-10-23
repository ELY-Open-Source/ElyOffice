//! Password prompts, zoomed pairs, jumps across (C11.1).

use ely_office::{Dialog, Dialogs as _, Opening as _, Tabs as _};
use gpui::{Entity, Modifiers, ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext, point, px};

use super::{file, frames, settle, shown, windowed};
use crate::layout::{Mode, Zoom};
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
