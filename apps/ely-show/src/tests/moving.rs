//! A live deck moves to a new window (W.1d).

use std::{cell::RefCell, rc::Rc};

use ely_office::{Core, moving::Moving as _};
use gpui::AnyWindowHandle;

use super::*;

/// The first window, built as the app builds it.
fn first(cx: &mut TestAppContext, dir: &Path) -> Option<(Entity<Deck>, AnyWindowHandle)> {
    // Real windows start recovery: never in the user's folder.
    ely_platform::test_root();
    let install = ely_lok::test_install()?;
    let (worker, stop) = ely_office::lok::test_worker(&install);
    let work = dir.join("work");
    std::fs::create_dir_all(&work).expect("the work dir");
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        ely_ui::settings::install_defaults(cx);
        ely_ui::install_theme(cx);
        ely_ui::i18n::install_as("en-US", cx);
        ely_ui::windows::allow_for_tests(ely_platform::AppId::Show, crate::window::more, cx);
    });
    let held = Rc::new(RefCell::new(None));
    let kept = Rc::clone(&held);
    let window = cx
        .update(|cx| {
            cx.open_window(Default::default(), move |window, cx| {
                let deck = crate::window::deck(move |cx| Core::new(worker, stop, work, cx), cx);
                kept.replace(Some(deck.clone()));
                crate::window::root(&deck, window, cx)
            })
        })
        .expect("a window");
    let window: AnyWindowHandle = window.into();
    cx.update(|cx| ely_ui::tray::testing::opened(ely_platform::AppId::Show, window, cx));
    let deck = held.take().expect("the deck");
    Some((deck, window))
}

/// Draws the windows until `done`; plain ticks draw nothing.
#[track_caller]
fn wait(cx: &mut TestAppContext, done: impl Fn(&gpui::App) -> bool) {
    // The worker answers in real time, not test time.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while std::time::Instant::now() < deadline {
        cx.run_until_parked();
        if cx.update(|app| done(app)) {
            return;
        }
        cx.executor().advance_clock(std::time::Duration::from_millis(20));
    }
    panic!("the windows never showed it");
}

/// A blank deck with its slide pictured; its tab id.
fn opened(cx: &mut TestAppContext, deck: &Entity<Deck>) -> u64 {
    let before = cx.update(|app| deck.read(app).core.tabs.len());
    deck.update(cx, |deck, cx| deck.new_blank(cx));
    wait(cx, |app| deck.read(app).core.tabs.len() > before);
    let id = cx.update(|app| deck.read(app).core.tabs.last().expect("a tab").id);
    wait(cx, |app| deck.read(app).tab(id).is_some_and(|tab| !tab.extra.thumbs.is_empty() && !tab.extra.painting));
    id
}

#[test]
fn a_deck_moves_with_its_pictures_and_history() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((deck, home)) = first(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    opened(&mut cx, &deck);
    let id = opened(&mut cx, &deck);
    deck.update(&mut cx, |deck, cx| deck.run_active("slide.insert", &[], cx));
    wait(&mut cx, |app| deck.read(app).tab(id).is_some_and(|tab| tab.extra.thumbs.len() == 2 && !tab.extra.painting));
    let doc = cx.update(|app| deck.read(app).tab(id).expect("the tab").doc.clone());
    cx.update(|app| ely_office::moving::to_new_window(deck.clone(), id, app));
    wait(&mut cx, |app| deck.read(app).tab(id).is_none());
    let other = cx.update(|app| crate::window::front(None, app)).expect("a front window");
    assert_ne!(other, deck, "a new window is in front");
    cx.update(|app| {
        let tab = other.read(app).tab(id).expect("the moved tab");
        assert_eq!(tab.doc, doc, "the same kernel document, not reopened");
        assert_eq!(tab.extra.thumbs.len(), 2, "its pictures came along");
        assert_eq!(other.read(app).core.tabs.len(), 1, "no blank added");
    });
    other.update(&mut cx, |other, cx| other.run_active("edit.undo", &[], cx));
    wait(&mut cx, |app| other.read(app).tab(id).is_some_and(|tab| tab.extra.thumbs.len() == 1));
    let last = cx.update(|app| deck.read(app).core.active()).expect("the other tab");
    cx.update(|app| ely_office::moving::to_new_window(deck.clone(), last, app));
    wait(&mut cx, |app| !app.windows().contains(&home));
}

#[test]
fn a_deck_being_shown_or_asked_about_stays() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((deck, _)) = first(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let id = opened(&mut cx, &deck);
    let stays = deck.update(&mut cx, |deck, cx| {
        deck.show_starting = true;
        let starting = deck.move_out(id, cx).err();
        deck.show_starting = false;
        starting
    });
    assert_eq!(stays, Some("office.move.dialog"), "a show starts from it");
    let stays = deck.update(&mut cx, |deck, cx| {
        deck.dialog = Some(crate::dialog::Dialog::Table { tab: id, columns: 2, rows: 2 });
        deck.move_out(id, cx).err()
    });
    assert_eq!(stays, Some("office.move.dialog"), "its table is asked");
}

#[test]
fn a_read_or_picture_in_flight_keeps_its_deck() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((deck, _)) = first(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let id = opened(&mut cx, &deck);
    let stays = deck.update(&mut cx, |deck, cx| {
        deck.repaint_all(id, cx);
        assert!(deck.tab(id).expect("the tab").extra.painting, "a picture is asked");
        deck.move_out(id, cx).err()
    });
    assert_eq!(stays, Some("office.move.busy"), "the picture lands here first");
    wait(&mut cx, |app| deck.read(app).tab(id).is_some_and(|tab| !tab.extra.painting));
    let stays = deck.update(&mut cx, |deck, cx| {
        deck.force_reread(id, cx);
        assert!(deck.tab(id).expect("the tab").extra.reading, "a read is asked");
        deck.move_out(id, cx).err()
    });
    assert_eq!(stays, Some("office.move.busy"), "the read lands here first");
}
