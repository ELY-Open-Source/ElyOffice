use ely_lok::LokView;
use ely_office::prelude::*;
use gpui::{Entity, ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, point};

use super::objects::{blank, selection};
use super::{counts, quiet, settle, update};
use crate::deck::Deck;

/// The slide canvas alone in a window.
struct Host(Entity<LokView>);

impl gpui::Render for Host {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};
        gpui::div().size_full().child(self.0.clone())
    }
}

/// The current slide once every turn landed.
fn current(shown: &mut VisualTestContext, deck: &Entity<Deck>) -> usize {
    quiet(shown, deck);
    shown.update(|_, app| deck.read(app).active_tab().map(|tab| tab.extra.slides.current).expect("a tab"))
}

/// Whole notches, in this platform's lines.
fn wheel(shown: &mut VisualTestContext, notches: f32) {
    let middle = shown.update(|window, _| window.viewport_size()).center();
    let delta = ScrollDelta::Lines(point(0.0, notches * ely_lok::notch()));
    shown.simulate_event(ScrollEvent { position: middle.into(), delta, ..Default::default() });
}

#[test]
fn the_wheel_and_up_down_turn_slides() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    for n in 2..=3 {
        update(&mut cx, &deck, |deck, cx| deck.object_command("slide.insert", &[], cx));
        settle(&cx, |app| counts(app, &deck).0 == n);
    }
    let view = cx.update(|app| deck.read(app).active_tab().expect("a tab").view.clone());
    cx.update(|app| view.update(app, |view, _| view.show_part(0)));
    let window = cx.add_window(|_, _| Host(view.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    shown.run_until_parked();
    assert_eq!(current(&mut shown, &deck), 0);
    wheel(&mut shown, -2.0);
    assert_eq!(current(&mut shown, &deck), 2, "two notches in one event turn two");
    wheel(&mut shown, 1.0);
    wheel(&mut shown, 1.0);
    assert_eq!(current(&mut shown, &deck), 0, "rapid notches each turn");
    wheel(&mut shown, -1.0);
    assert_eq!(current(&mut shown, &deck), 1, "the wheel turns down");
    shown.update(|window, app| {
        let focus = view.read(app).focus_handle().clone();
        window.focus(&focus, app)
    });
    shown.simulate_keystrokes("down");
    assert_eq!(current(&mut shown, &deck), 2, "Down turns");
    shown.simulate_keystrokes("down");
    assert_eq!(current(&mut shown, &deck), 2, "the last slide stays");
    shown.simulate_keystrokes("up");
    assert_eq!(current(&mut shown, &deck), 1, "Up turns back");
    wheel(&mut shown, 1.0);
    assert_eq!(current(&mut shown, &deck), 0, "the wheel turns up");
    // Down queued behind Tab moves the object Tab picks.
    shown.simulate_keystrokes("tab down");
    assert_eq!(current(&mut shown, &deck), 0, "Down after Tab moves, not turns");
    assert!(shown.update(|_, app| selection(app, &deck)).is_some(), "Tab picked an object");
    // A turn lands before the keys queued after it.
    shown.simulate_keystrokes("escape down tab down");
    assert_eq!(current(&mut shown, &deck), 1, "Down turns, then moves");
    assert!(shown.update(|_, app| selection(app, &deck)).is_some(), "Tab picked on the new slide");
}
