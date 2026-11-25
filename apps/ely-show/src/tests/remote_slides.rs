//! Another program lists and restructures slides; the view stays.

use ely_office::prelude::*;
use gpui::{Entity, TestAppContext};

use super::remote::{DECK, ask, current, marked};
use super::{quiet, settle, update};
use crate::deck::Deck;

/// `slide list` as names and layouts.
fn listed(cx: &mut TestAppContext, deck: &Entity<Deck>, file: &str) -> Vec<(String, String)> {
    let json: serde_json::Value = serde_json::from_str(&ask(cx, deck, &["slide.list", file]).expect("listed")).expect("JSON");
    let items = json.as_array().expect("an array");
    for (at, item) in items.iter().enumerate() {
        assert_eq!(item["number"].as_u64(), Some(at as u64 + 1), "{item}");
        assert!(item["hidden"].is_boolean() && item["title"].is_string(), "{item}");
    }
    let text = |item: &serde_json::Value, key: &str| item[key].as_str().expect("text").to_owned();
    items.iter().map(|item| (text(item, "name"), text(item, "layout"))).collect()
}

/// Names only, in order.
fn names(cx: &mut TestAppContext, deck: &Entity<Deck>, file: &str) -> Vec<String> {
    listed(cx, deck, file).into_iter().map(|(name, _)| name).collect()
}

/// The shown slide, once ElyShow reads "Two" at `two`.
fn shown(cx: &mut TestAppContext, deck: &Entity<Deck>, count: usize, two: usize) -> Option<usize> {
    settle(cx, |app| {
        deck.read(app).active_tab().is_some_and(|tab| {
            let list = &tab.extra.slides.list;
            list.len() == count && list[two].name == "Two"
        })
    });
    quiet(cx, deck);
    current(cx, deck)
}

/// Runs `edit.undo` and waits.
fn undo(cx: &mut TestAppContext, deck: &Entity<Deck>) {
    update(cx, deck, |deck, cx| deck.run_active("edit.undo", &[], cx));
    quiet(cx, deck);
}

#[test]
fn slides_are_listed_added_duplicated_moved_and_deleted_while_the_view_stays() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = super::deck(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let path = dir.path().join("deck.fodp");
    std::fs::write(&path, DECK).expect("writes the deck");
    update(&mut cx, &deck, |deck, cx| deck.open_file(path.clone(), cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.slides.list.len() == 2));
    quiet(&cx, &deck);
    let file = path.display().to_string();
    assert_eq!(names(&mut cx, &deck, &file), ["One", "Two"]);
    // The user looks at "Two" with its table picked.
    update(&mut cx, &deck, |deck, cx| deck.go_to_finding(1, cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.slides.current == 1));
    update(&mut cx, &deck, |deck, cx| deck.select_objects(&["0".into()], cx));
    settle(&cx, |app| marked(app, &deck) == vec![0]);

    assert_eq!(ask(&mut cx, &deck, &["slide.add", &file, "1", "0"]), Ok(String::new()));
    quiet(&cx, &deck);
    let now = listed(&mut cx, &deck, &file);
    assert_eq!((now.len(), now[0].1.as_str(), now[2].0.as_str()), (3, "title", "Two"), "{now:?}");
    assert_eq!(shown(&mut cx, &deck, 3, 2), Some(2), "still showing Two");
    assert_eq!(cx.update(|app| marked(app, &deck)), vec![0], "the pick stays");
    undo(&mut cx, &deck);
    assert_eq!(names(&mut cx, &deck, &file), ["One", "Two"], "one undo step");
    assert_eq!(shown(&mut cx, &deck, 2, 1), Some(1), "undo leaves Two shown");

    assert_eq!(ask(&mut cx, &deck, &["slide.add", &file, "", "1"]), Ok(String::new()));
    assert_eq!(listed(&mut cx, &deck, &file)[2].1, "title-content", "the end, the default layout");
    assert_eq!(shown(&mut cx, &deck, 3, 1), Some(1), "still showing Two");
    // Undo removes the shown slide: its neighbour shows.
    update(&mut cx, &deck, |deck, cx| deck.go_to_finding(2, cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.slides.current == 2));
    undo(&mut cx, &deck);
    assert_eq!(shown(&mut cx, &deck, 2, 1), Some(1), "the neighbour, not the first");

    assert_eq!(ask(&mut cx, &deck, &["slide.duplicate", &file, "1"]), Ok(String::new()));
    let now = names(&mut cx, &deck, &file);
    assert_eq!((now.len(), now[0].as_str(), now[2].as_str()), (3, "One", "Two"), "{now:?}");
    assert_eq!(shown(&mut cx, &deck, 3, 2), Some(2), "Two moved down a place");
    undo(&mut cx, &deck);
    assert_eq!(shown(&mut cx, &deck, 2, 1), Some(1), "undo leaves Two shown");

    assert_eq!(ask(&mut cx, &deck, &["slide.move", &file, "1", "2"]), Ok(String::new()));
    assert_eq!(names(&mut cx, &deck, &file), ["Two", "One"]);
    assert_eq!(shown(&mut cx, &deck, 2, 0), Some(0), "Two, now first");
    assert_eq!(ask(&mut cx, &deck, &["slide.move", &file, "2", "2"]), Ok(String::new()), "onto itself");
    undo(&mut cx, &deck);
    assert_eq!(names(&mut cx, &deck, &file), ["One", "Two"], "one undo step");

    for (words, why) in [
        (&["slide.delete", &file, "3"][..], "slide 3 does not exist (must be 1 to 2)"),
        (&["slide.add", &file, "4", "1"], "position 4 is out of range (must be 1 to 3)"),
        (&["slide.move", &file, "1", "3"], "position 3 is out of range (must be 1 to 2)"),
    ] {
        assert_eq!(ask(&mut cx, &deck, words), Err(why.into()), "{words:?}");
    }
    assert_eq!(names(&mut cx, &deck, &file), ["One", "Two"], "refusals change nothing");

    // Deleting the shown slide shows its neighbour.
    assert_eq!(ask(&mut cx, &deck, &["slide.delete", &file, "2"]), Ok(String::new()));
    assert_eq!(names(&mut cx, &deck, &file), ["One"]);
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.slides.list.len() == 1));
    quiet(&cx, &deck);
    assert_eq!(current(&mut cx, &deck), Some(0));
    assert_eq!(ask(&mut cx, &deck, &["slide.delete", &file, "1"]), Err("cannot delete the only slide".into()));
    assert!(ask(&mut cx, &deck, &["save", &file]).is_ok(), "saved");
    let on_disk = std::fs::read_to_string(&path).expect("reads the saved deck");
    assert_eq!(on_disk.matches("<draw:page ").count(), 1, "one slide on disk");
}
