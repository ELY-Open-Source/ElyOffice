use gpui::{Entity, ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, point, px};

use super::objects::blank;
use super::{counts, settle, update};
use crate::slides::Strip;

/// The slide strip alone in a window.
struct Host(Entity<Strip>);

impl gpui::Render for Host {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};
        gpui::div().size_full().child(self.0.clone())
    }
}

#[test]
fn the_wheel_scrolls_the_strip_to_its_last_slide() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    for n in 2..=24 {
        update(&mut cx, &deck, |deck, cx| deck.object_command("slide.insert", &[], cx));
        settle(&cx, |app| counts(app, &deck).0 == n);
    }
    let strip = cx.update(|app| deck.read(app).strip.clone());
    let window = cx.add_window(|_, _| Host(strip));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    shown.run_until_parked();
    let height = shown.update(|window, _| window.viewport_size().height);
    let last = |shown: &mut VisualTestContext| shown.debug_bounds("show-slide-23").expect("the last slide is laid out");
    let before = last(&mut shown);
    assert!(before.bottom() > height, "the strip is longer than the window");
    let at = point(before.left() + px(20.0), height / 2.0);
    for _ in 0..20 {
        shown.simulate_event(ScrollEvent {
            position: at,
            delta: ScrollDelta::Pixels(point(px(0.0), px(-1000.0))),
            ..Default::default()
        });
        shown.run_until_parked();
    }
    let after = last(&mut shown);
    assert!(after.bottom() <= height, "the last slide is in view: {after:?}");
}
