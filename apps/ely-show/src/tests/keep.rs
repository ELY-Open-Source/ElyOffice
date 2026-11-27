//! Countdowns and scoreboards come back with the saved deck.

use std::time::Duration;

use ely_files::SaveState;
use ely_office::prelude::*;
use gpui::TestAppContext;

use super::link::rooted;
use super::slides::in_window;
use super::play::three_slides;
use super::{quiet, settle, update};
use crate::slideshow::scoreboard::Source;
use crate::slideshow::timer::{Alarm, Spot};

#[test]
fn a_saved_deck_brings_back_its_countdowns_and_scoreboards() {
    for format in ["pptx", "odp"] {
        let dir = tempfile::tempdir().expect("a temp dir");
        let mut cx = TestAppContext::single();
        let Some((deck, _)) = rooted(&mut cx, dir.path()) else {
            eprintln!("SKIP: no LOK build");
            return;
        };
        three_slides(&mut cx, &deck);
        quiet(&cx, &deck);
        let id = cx.update(|app| deck.read(app).core.active()).expect("a tab");
        let state = |cx: &TestAppContext| cx.update(|app| deck.read(app).active_tab().map(|tab| tab.session.state()));
        let source = Source { book: String::new(), path: Some("/games/cup.ods".into()), name: "cup.ods".into(), sheet: "Final".into() };
        update(&mut cx, &deck, |deck, cx| {
            let tab = deck.tab_mut(id).expect("the tab");
            let names: Vec<String> = tab.extra.slides.list.iter().map(|slide| slide.name.clone()).collect();
            tab.extra.scoreboards.insert(names[2].clone(), source.clone());
            deck.set_timer(id, Some(names[1].clone()), Some(Duration::from_secs(90)), (Spot::Corner, None, Alarm::QUIET), cx);
            deck.drag_timer(id, &names[1], (0.25, 0.75), cx);
        });
        let copy = dir.path().join(format!("copy.{format}"));
        in_window(&mut cx, &deck, |deck, window, cx| deck.write_as(id, copy.clone(), None, window, cx));
        let named = format!("copy.{format}");
        settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.name == named.as_str() && !tab.exporting));
        quiet(&cx, &deck);
        assert_eq!(state(&cx), Some(SaveState::Saved), "{format}");
        update(&mut cx, &deck, |deck, cx| deck.close_tab(id, cx));
        settle(&cx, |app| deck.read(app).core.tabs.is_empty());
        update(&mut cx, &deck, |deck, cx| deck.open_with(copy.clone(), String::new(), cx));
        settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.slides.list.len() == 3));
        quiet(&cx, &deck);
        let id = cx.update(|app| deck.read(app).core.active()).expect("a tab");
        update(&mut cx, &deck, |deck, cx| {
            deck.place_kept(id);
            deck.stay_unsaved(id, cx);
        });
        assert_eq!(state(&cx), Some(SaveState::Saved), "{format}: opening changes nothing");
        let (name, timer, score) = cx.update(|app| {
            let extra = &deck.read(app).tab(id).expect("the tab").extra;
            let names: Vec<&String> = extra.slides.list.iter().map(|slide| &slide.name).collect();
            assert_eq!((extra.timers.len(), extra.scoreboards.len()), (1, 1), "{format}");
            (names[1].clone(), extra.timers[names[1]], extra.scoreboards[names[2]].clone())
        });
        assert_eq!((timer.length.as_secs(), timer.spot, timer.at), (90, Spot::Corner, Some((0.25, 0.75))));
        assert_eq!(score, source, "{format}");
        // A move alone leaves the deck unsaved.
        update(&mut cx, &deck, |deck, cx| deck.drag_timer(id, &name, (0.5, 0.5), cx));
        assert_eq!(state(&cx), Some(SaveState::Dirty), "{format}");
        // Until a save lands, an undo to saved stays unsaved.
        let undone = |cx: &mut TestAppContext| {
            update(cx, &deck, |deck, cx| {
                deck.tab_mut(id).expect("the tab").session.reverted();
                deck.stay_unsaved(id, cx);
            });
            state(cx)
        };
        let ask: crate::slideshow::scoreboard::Ask = |words| Err(std::io::Error::other(format!("{words:?}")));
        update(&mut cx, &deck, |deck, cx| assert!(deck.keeping(id, format, ask, cx).is_some(), "{format}: a save starts"));
        assert_eq!(undone(&mut cx), Some(SaveState::Dirty), "{format}: that save did not land");
        let again = dir.path().join(format!("again.{format}"));
        in_window(&mut cx, &deck, |deck, window, cx| deck.write_as(id, again.clone(), None, window, cx));
        let named = format!("again.{format}");
        settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.name == named.as_str() && !tab.exporting));
        quiet(&cx, &deck);
        assert_eq!(undone(&mut cx), Some(SaveState::Saved), "{format}: a landed save keeps the move");
    }
}

#[test]
fn another_programs_comment_is_kept_and_never_written_over() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((deck, _)) = rooted(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    quiet(&cx, &deck);
    let active = |cx: &TestAppContext| cx.update(|app| deck.read(app).core.active()).expect("a tab");
    let state = |cx: &TestAppContext| cx.update(|app| deck.read(app).active_tab().map(|tab| tab.session.state()));
    let save_as = |cx: &mut TestAppContext, path: &std::path::Path| {
        let id = active(cx);
        let path = path.to_path_buf();
        in_window(cx, &deck, |deck, window, cx| deck.write_as(id, path, None, window, cx));
        settle(cx, |app| deck.read(app).active_tab().is_some_and(|tab| !tab.exporting));
        quiet(cx, &deck);
    };
    let theirs = dir.path().join("theirs.pptx");
    save_as(&mut cx, &theirs);
    let id = active(&cx);
    update(&mut cx, &deck, |deck, cx| deck.close_tab(id, cx));
    settle(&cx, |app| deck.read(app).core.tabs.is_empty());
    // The kernel's package ends with an empty comment.
    let mut bytes = std::fs::read(&theirs).expect("the saved deck");
    let end = bytes.len() - 22;
    assert_eq!((&bytes[end..end + 4], &bytes[end + 20..]), (&b"PK\x05\x06"[..], &[0, 0][..]));
    bytes[end + 20..].copy_from_slice(&6_u16.to_le_bytes());
    bytes.extend_from_slice(b"Theirs");
    std::fs::write(&theirs, &bytes).expect("another program's comment");
    update(&mut cx, &deck, |deck, cx| deck.open_with(theirs.clone(), String::new(), cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.slides.list.len() == 3));
    quiet(&cx, &deck);
    let again = dir.path().join("again.pptx");
    save_as(&mut cx, &again);
    let kept = std::fs::read(&again).expect("saved again");
    assert!(kept.ends_with(b"Theirs"), "a save with nothing of ours keeps theirs");
    assert_eq!(state(&cx), Some(SaveState::Saved));
    update(&mut cx, &deck, |deck, cx| {
        let id = deck.core.active().expect("a tab");
        let first = deck.tab(id).expect("the tab").extra.slides.list[0].name.clone();
        deck.set_timer(id, Some(first), Some(Duration::from_secs(60)), (Spot::Top, None, Alarm::QUIET), cx);
    });
    save_as(&mut cx, &again);
    assert_eq!(std::fs::read(&again).expect("still there"), kept, "theirs is never written over");
    assert_ne!(state(&cx), Some(SaveState::Saved), "the countdown stays unsaved");
}
