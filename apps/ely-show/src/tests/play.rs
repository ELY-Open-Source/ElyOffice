//! The show: keys, hidden slides, black, laser (C10.5).

use ely_office::prelude::*;
use gpui::{Entity, Keystroke, TestAppContext};

use super::objects::{act, blank};
use super::{settle, update};
use crate::deck::Deck;
use crate::slideshow::{Show, steps::Place};

fn press(cx: &mut TestAppContext, show: &Entity<Show>, key: &'static str) -> bool {
    let key = Keystroke::parse(key).expect("a key");
    cx.update(|app| show.update(app, |show, cx| show.key(&key, cx)))
}

/// Keys typed into the show's own window.
pub(super) fn typed(cx: &mut TestAppContext, show: &Entity<Show>, keys: &str) {
    let window = cx.update(|app| show.read(app).windows[0]);
    cx.simulate_keystrokes(window, keys);
}

pub(super) fn place(cx: &mut TestAppContext, show: &Entity<Show>) -> Place {
    cx.update(|app| show.read(app).place)
}

/// Slide two: click effect, notes. Three: hidden.
pub(super) fn three_slides(cx: &mut TestAppContext, deck: &Entity<Deck>) {
    let slides = |app: &gpui::App| {
        let tab = deck.read(app).active_tab();
        tab.map(|tab| tab.extra.slides.list.iter().map(|s| s.visible).collect::<Vec<_>>())
    };
    update(cx, deck, |deck, cx| deck.design_command("slide.insert", &[], cx));
    settle(cx, |app| slides(app).is_some_and(|list| list.len() == 2));
    act(cx, deck, ("draw.shape", &["rectangle", "true"]), |o| {
        o.objects.len() == 3
    });
    act(cx, deck, ("object.select", &["2", "false"]), |o| {
        o.selection.is_some()
    });
    let add = serde_json::json!({ "preset": "ooo-entrance-fade-in" }).to_string();
    let effects = |app: &gpui::App| {
        let tab = deck.read(app).active_tab();
        tab.and_then(|tab| tab.extra.motion.as_ref().map(|(_, e)| e.len()))
    };
    update(cx, deck, |deck, cx| deck.design_command("animation.add", &[&add], cx));
    settle(cx, |app| effects(app) == Some(1));
    update(cx, deck, |deck, cx| {
        deck.design_command("notes.set", &["Say hello", "1"], cx);
        deck.design_command("slide.insert", &[], cx);
    });
    settle(cx, |app| slides(app).is_some_and(|list| list.len() == 3));
    update(cx, deck, |deck, cx| deck.design_command("slide.hide", &[], cx));
    settle(cx, |app| slides(app) == Some(vec![true, true, false]));
}

#[test]
fn a_show_plays_clicks_skips_hidden_and_ends() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    update(&mut cx, &deck, |deck, cx| deck.start_show(false, cx));
    settle(&cx, |app| deck.read(app).show.is_some());
    let show = cx.update(|app| deck.read(app).show.clone().expect("playing"));
    settle(&cx, |app| {
        let show = show.read(app);
        show.stack(0).is_some() && show.stack(1).is_some()
    });
    let windows = cx.update(|app| show.read(app).windows.len());
    assert_eq!(windows, 1, "one screen: no presenter view");
    assert_eq!(place(&mut cx, &show), Place::Slide { part: 0, clicks: 0 });
    typed(&mut cx, &show, "right");
    assert_eq!(place(&mut cx, &show), Place::Slide { part: 1, clicks: 0 });
    let notes = cx.update(|app| show.read(app).slides[1].notes.clone());
    assert_eq!(notes, "Say hello");
    press(&mut cx, &show, "space");
    assert_eq!(place(&mut cx, &show), Place::Slide { part: 1, clicks: 1 });
    press(&mut cx, &show, "right");
    assert_eq!(place(&mut cx, &show), Place::End, "the hidden slide is passed");
    press(&mut cx, &show, "left");
    assert_eq!(place(&mut cx, &show), Place::Slide { part: 1, clicks: 1 });
    press(&mut cx, &show, "1");
    press(&mut cx, &show, "enter");
    assert_eq!(place(&mut cx, &show), Place::Slide { part: 0, clicks: 0 });
    press(&mut cx, &show, "b");
    press(&mut cx, &show, "l");
    assert!(cx.update(|app| show.read(app).black && show.read(app).laser));
    assert!(!press(&mut cx, &show, "cmd-q"), "app keys pass through");
    let before = cx.update(|app| app.windows().len());
    typed(&mut cx, &show, "escape");
    settle(&cx, |app| deck.read(app).show.is_none());
    assert_eq!(cx.update(|app| app.windows().len()), before - 1, "the show window closes");
}

#[test]
fn from_here_starts_on_the_shown_slide_even_hidden() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    update(&mut cx, &deck, |deck, cx| deck.start_show(true, cx));
    settle(&cx, |app| deck.read(app).show.is_some());
    let show = cx.update(|app| deck.read(app).show.clone().expect("playing"));
    assert_eq!(place(&mut cx, &show), Place::Slide { part: 2, clicks: 0 });
    press(&mut cx, &show, "right");
    assert_eq!(place(&mut cx, &show), Place::End);
    press(&mut cx, &show, "right");
    settle(&cx, |app| deck.read(app).show.is_none());
}

#[test]
fn a_click_on_media_finds_it_and_elsewhere_does_not() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let clip = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/media/clip.mp4");
    let url = ely_lok::file_url(&clip);
    update(&mut cx, &deck, |deck, cx| {
        deck.design_command("media.insert", &[&url, "false", "8000", "4500"], cx)
    });
    settle(&cx, |app| {
        deck.read(app).active_tab().is_some_and(|tab| tab.extra.media.len() == 1)
    });
    update(&mut cx, &deck, |deck, cx| deck.start_show(true, cx));
    settle(&cx, |app| deck.read(app).show.is_some());
    let show = cx.update(|app| deck.read(app).show.clone().expect("playing"));
    let (bounds, size) = cx.update(|app| {
        let show = show.read(app);
        (show.slides[0].media[0].bounds, show.slide)
    });
    let centre = (
        (bounds.0 + bounds.2 / 2) as f32 / size.0,
        (bounds.1 + bounds.3 / 2) as f32 / size.1,
    );
    assert_eq!(cx.update(|app| show.read(app).media_at(centre)), Some(0));
    assert_eq!(cx.update(|app| show.read(app).media_at((0.01, 0.01))), None);
}

#[test]
fn a_second_start_says_the_show_is_busy() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    // The window's toasts live on its root.
    let root = super::slides::in_window(&mut cx, &deck, |deck, window, cx| {
        let this = cx.entity();
        let root = ely_ui::document_root(ely_platform::AppId::Show, this.into(), &ely_ui::commands::FILE, Vec::new(), window, cx);
        deck.core.root = root.downgrade();
        root
    });
    update(&mut cx, &deck, |deck, cx| {
        deck.show_starting = true;
        deck.start_show(false, cx);
    });
    cx.run_until_parked();
    let (toasts, busy) = cx.update(|app| (root.read(app).toasts(app), Deck::text(app, "show.play.busy", &[])));
    assert!(toasts.iter().any(|(text, _)| *text == busy), "{toasts:?}");
}

#[test]
fn a_start_left_for_another_tab_says_so() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let first = cx.update(|app| deck.read(app).core.active.expect("a tab"));
    update(&mut cx, &deck, |deck, cx| deck.new_blank(cx));
    settle(&cx, |app| deck.read(app).core.tabs.len() == 2 && deck.read(app).core.active != Some(first));
    let second = cx.update(|app| deck.read(app).core.active.expect("a tab"));
    let root = super::slides::in_window(&mut cx, &deck, |deck, window, cx| {
        let this = cx.entity();
        let root = ely_ui::document_root(ely_platform::AppId::Show, this.into(), &ely_ui::commands::FILE, Vec::new(), window, cx);
        deck.core.root = root.downgrade();
        root
    });
    update(&mut cx, &deck, |deck, cx| {
        deck.core.active = Some(first);
        deck.start_show(false, cx);
        // Another tab comes forward while the copy is read.
        deck.core.active = Some(second);
    });
    let gone = cx.update(|app| Deck::text(app, "show.play.gone", &[]));
    settle(&cx, |app| root.read(app).toasts(app).iter().any(|(text, _)| *text == gone));
    assert!(cx.update(|app| !deck.read(app).show_starting && deck.read(app).show.is_none()));
}
