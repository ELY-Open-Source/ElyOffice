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
    }
}
