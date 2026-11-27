//! #125: a click on a change opens its card.

use gpui::{Modifiers, MouseButton, VisualTestContext};

use super::*;

/// The page with its card, as the window draws it.
struct Host(Entity<Review>, Entity<ely_lok::LokView>);

impl gpui::Render for Host {
    fn render(&mut self, _: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};
        let card = self.0.update(cx, |review, cx| review.card(cx));
        let page = gpui::div().relative().size_full().child(self.1.clone());
        watch(&self.0, page).children(card)
    }
}

fn frames(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
}

/// Waits for the card's state, frames drawn between.
fn open_is(cx: &mut VisualTestContext, review: &Entity<Review>, want: Option<u64>) {
    for _ in 0..3000 {
        frames(cx);
        if cx.update(|_, app| review.read(app).card.open) == want {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("the card is not {want:?}");
}

#[test]
fn a_click_on_the_changes_own_caret_opens_its_card_again_after_escape() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    blank(&mut cx, &docs);
    let review = cx.update(|app| docs.read(app).review.clone());
    cx.update(|app| docs.update(app, |docs, cx| docs.side_view(cx)));
    type_into(&mut cx, &docs, "Start");
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_track(&TrackChanges, window, cx));
    type_into(&mut cx, &docs, " Alpha Beta");
    settle(&cx, |app| review.read(app).changes.len() == 1);
    let view = cx.update(|app| docs.read(app).active_tab().expect("a tab").view.clone());
    let index = cx.update(|app| review.read(app).changes[0].index);
    let window = cx.add_window(|_, _| Host(review.clone(), view.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    // Typing left the caret inside the change; no card.
    open_is(&mut shown, &review, None);
    let at = shown.update(|_, app| view.read(app).cursor_bounds().expect("a caret").center());
    let click = |shown: &mut VisualTestContext| {
        shown.simulate_mouse_move(at, None, Modifiers::none());
        shown.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
        shown.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    };
    click(&mut shown);
    open_is(&mut shown, &review, Some(index));
    shown.simulate_keystrokes("escape");
    open_is(&mut shown, &review, None);
    // The same spot again: the caret stays, the card opens.
    click(&mut shown);
    open_is(&mut shown, &review, Some(index));
}
