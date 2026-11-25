//! A picture placed on a slide by number.

use ely_office::prelude::*;
use gpui::TestAppContext;

use super::remote::{DECK, ask, current, marked};
use super::{quiet, settle, update};

/// Picture frames on page `name`, tables excluded.
fn pictures(xml: &str, name: &str) -> Vec<String> {
    let page = &xml[xml.find(&format!("<draw:page draw:name=\"{name}\"")).expect("the page")..];
    let page = &page[..page.find("</draw:page>").expect("its end")];
    page.split("<draw:frame")
        .skip(1)
        .filter(|frame| frame.contains("<draw:image") && !frame.contains("<table:table"))
        .map(|frame| frame[..frame.find('>').expect("a tag")].to_owned())
        .collect()
}

#[test]
fn pictures_land_on_a_slide_by_number_while_the_view_stays() {
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
    // The user looks at "Two" with its table picked.
    update(&mut cx, &deck, |deck, cx| deck.go_to_finding(1, cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.slides.current == 1));
    update(&mut cx, &deck, |deck, cx| deck.select_objects(&["0".into()], cx));
    settle(&cx, |app| marked(app, &deck) == vec![0]);

    let picture = dir.path().join("red.svg");
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="200pt" height="100pt"><rect width="100%" height="100%" fill="red"/></svg>"#;
    std::fs::write(&picture, svg).expect("a picture");
    let insert = |cx: &mut TestAppContext, slide: &str, picture: &std::path::Path, width: &str| {
        let shown = picture.display().to_string();
        ask(cx, &deck, &["image.insert", &file, slide, &ely_lok::file_url(picture), width, &shown])
    };
    assert_eq!(insert(&mut cx, "1", &picture, ""), Ok(String::new()));
    assert_eq!(insert(&mut cx, "2", &picture, "4535"), Ok(String::new()));
    quiet(&cx, &deck);
    assert_eq!(current(&mut cx, &deck), Some(1), "still showing Two");
    assert_eq!(cx.update(|app| marked(app, &deck)), vec![0], "the pick stays");

    let fake = dir.path().join("fake.png");
    std::fs::write(&fake, "not a picture").expect("a file");
    let unread = format!("{}: cannot read as an image; nothing was inserted", fake.display());
    assert_eq!(insert(&mut cx, "1", &fake, ""), Err(unread));
    assert_eq!(insert(&mut cx, "3", &picture, ""), Err("slide 3 does not exist (must be 1 to 2)".into()));
    assert_eq!(current(&mut cx, &deck), Some(1), "refusals leave the view");
    assert!(ask(&mut cx, &deck, &["save", &file]).is_ok(), "saved");
    let on_disk = std::fs::read_to_string(&path).expect("reads the saved deck");
    let (one, two) = (pictures(&on_disk, "One"), pictures(&on_disk, "Two"));
    assert_eq!((one.len(), two.len()), (1, 1), "one each, refusals add none: {one:?} {two:?}");
    let width = &two[0][two[0].find("svg:width=\"").expect("a width") + 11..];
    let width: f64 = width[..width.find("cm").expect("in cm")].parse().expect("a number");
    assert!((width - 8.0).abs() < 0.01, "--width 8cm: {two:?}");
}
