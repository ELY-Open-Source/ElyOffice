//! Text and notes set by slide number; the view stays.

use ely_office::prelude::*;
use gpui::TestAppContext;

use super::remote::{DECK, ask, current, marked};
use super::{quiet, settle, update};

#[test]
fn titles_bodies_and_notes_are_set_by_number_while_the_view_stays() {
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
    // Slide 3 has a title and a body.
    assert_eq!(ask(&mut cx, &deck, &["slide.add", &file, "", "1"]), Ok(String::new()));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.slides.list.len() == 3));
    quiet(&cx, &deck);
    // The user looks at "Two" with its table picked.
    update(&mut cx, &deck, |deck, cx| deck.go_to_finding(1, cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.slides.current == 1));
    update(&mut cx, &deck, |deck, cx| deck.select_objects(&["0".into()], cx));
    settle(&cx, |app| marked(app, &deck) == vec![0]);

    assert_eq!(ask(&mut cx, &deck, &["set", &file, "3", "title", "Plan"]), Ok(String::new()));
    assert_eq!(ask(&mut cx, &deck, &["set", &file, "3", "body", "One\r\nTwo"]), Ok(String::new()));
    assert_eq!(ask(&mut cx, &deck, &["notes.set", &file, "3", "Say hi\nthen go"]), Ok(String::new()));
    quiet(&cx, &deck);
    assert_eq!(current(&mut cx, &deck), Some(1), "still showing Two");
    assert_eq!(cx.update(|app| marked(app, &deck)), vec![0], "the pick stays");
    let text = ask(&mut cx, &deck, &["get", &file]).expect("the text");
    assert!(text.contains("Plan") && text.contains("One\nTwo"), "{text:?}");
    assert_eq!(ask(&mut cx, &deck, &["notes.get", &file, "3"]), Ok("Say hi\nthen go".into()));
    assert_eq!(ask(&mut cx, &deck, &["notes.get", &file, "2"]), Ok(String::new()), "no notes");

    // A break in WITH starts a paragraph; FIND holds none.
    assert_eq!(ask(&mut cx, &deck, &["replace", &file, "Plan", "Plan\r\nB"]), Ok(String::new()));
    let text = ask(&mut cx, &deck, &["get", &file]).expect("the text");
    assert!(text.contains("Plan\nB"), "{text:?}");
    assert!(ask(&mut cx, &deck, &["replace", &file, "Plan\nB", "x"]).is_err(), "FIND spans paragraphs");

    for (words, why) in [
        (&["set", &file, "1", "title", "x"][..], "slide 1 has no title placeholder"),
        (&["set", &file, "4", "body", "x"], "slide 4 does not exist (must be 1 to 3)"),
        (&["notes.get", &file, "4"], "slide 4 does not exist (must be 1 to 3)"),
        (&["notes.set", &file, "0", "x"], "slide 0 does not exist (must be 1 to 3)"),
    ] {
        assert_eq!(ask(&mut cx, &deck, words), Err(why.into()), "{words:?}");
    }
    assert_eq!(current(&mut cx, &deck), Some(1), "refusals leave the view");
    assert!(ask(&mut cx, &deck, &["save", &file]).is_ok(), "saved");
    let on_disk = std::fs::read_to_string(&path).expect("reads the saved deck");
    for text in ["Plan", "One", "Two", "Say hi", "then go"] {
        assert!(on_disk.contains(&format!(">{text}</text:p>")), "{text} on disk as its own paragraph");
    }
}
