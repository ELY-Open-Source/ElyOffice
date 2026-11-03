//! The keys follow the front deck across tab switches (R15.40).

use ely_gpui_component::theme::Theme;
use ely_office::prelude::*;
use gpui::{Entity, TestAppContext, VisualTestContext};

use super::objects::blank;
use super::{counts, settle, update};
use crate::deck::Deck;

/// The whole window, as the app draws it.
struct Front(Entity<Deck>);

impl gpui::Render for Front {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        self.0.clone()
    }
}

fn frames(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
}

fn holds_keys(cx: &mut VisualTestContext, deck: &Entity<Deck>, id: u64) -> bool {
    cx.update(|window, app| deck.read(app).tab(id).expect("open").view.read(app).focus_handle().is_focused(window))
}

/// One typed key must reach deck `id` alone.
#[track_caller]
fn first_key_lands(cx: &mut VisualTestContext, deck: &Entity<Deck>, id: u64) {
    let sent = |cx: &mut VisualTestContext| {
        cx.update(|_, app| deck.read(app).core.tabs.iter().map(|tab| (tab.id, tab.view.read(app).sent())).collect::<Vec<_>>())
    };
    let before = sent(cx);
    cx.simulate_keystrokes("right");
    let after = sent(cx);
    let grew: Vec<u64> = (before.iter().zip(&after))
        .filter(|(old, new)| new.1 > old.1)
        .map(|(_, new)| new.0)
        .collect();
    assert_eq!(grew, vec![id], "the first key reaches deck {id}");
}

#[test]
fn every_switch_hands_the_keys_to_the_front_deck() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let a = cx.update(|app| deck.read(app).core.active()).expect("a tab");
    update(&mut cx, &deck, |deck, cx| deck.new_blank(cx));
    settle(&cx, |app| deck.read(app).core.active() != Some(a) && counts(app, &deck) == (1, 1));
    let b = cx.update(|app| deck.read(app).core.active()).expect("b tab");
    cx.update(|cx| {
        Theme::update(cx, |theme| theme.reduced_motion = true);
        ely_ui::set_document_menus(ely_platform::AppId::Show, &ely_ui::commands::FILE, Vec::new(), cx);
    });
    let window = cx.add_window(|_, _| Front(deck.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    // Focus events need the window in front, as typing does.
    shown.update(|window, _| window.activate_window());
    shown.run_until_parked();
    assert!(shown.update(|window, _| window.is_window_active()), "the window is in front");
    shown.update(|window, app| deck.update(app, |deck, cx| {
        deck.own_window(window, cx);
        let focus = deck.tab(b).expect("b").view.read(cx).focus_handle().clone();
        window.focus(&focus, cx);
    }));
    frames(&mut shown);
    assert!(holds_keys(&mut shown, &deck, b), "the new deck has the keys");
    for id in [a, b] {
        // What a click on the tab strip runs.
        shown.update(|window, app| deck.update(app, |deck, cx| deck.show_tab(id, window, cx)));
        frames(&mut shown);
        first_key_lands(&mut shown, &deck, id);
    }
    // A switch that never touches focus.
    shown.update(|_, app| deck.update(app, |deck, cx| {
        deck.core.set_active(Some(a));
        cx.notify();
    }));
    frames(&mut shown);
    first_key_lands(&mut shown, &deck, a);
    shown.update(|_, app| deck.update(app, |deck, cx| deck.close_tab(a, cx)));
    frames(&mut shown);
    first_key_lands(&mut shown, &deck, b);
}
