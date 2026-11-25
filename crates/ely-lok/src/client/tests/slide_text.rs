//! A slide's title, body and notes by index (patch 0158).

use super::analysis::steps;
use super::show::open_deck;
use super::*;
use crate::show;

fn listed(worker: &mut Worker, doc: &Doc) -> Vec<show::Slide> {
    show::slides(&worker.values(doc, show::SLIDES).expect("reads")).expect("parses")
}

fn has(slide: &show::Slide, name: &str) -> bool {
    slide.boxes.iter().any(|(box_name, _)| box_name == name)
}

#[test]
fn titles_and_bodies_set_by_index_in_one_step_and_the_view_stays() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_deck();
    // Slide 3 is title-content; slide 4 is title-subtitle.
    worker.run(&doc, "slide.at", &["add", "2", "1"]).expect("adds");
    worker.run(&doc, "slide.at", &["add", "3", "0"]).expect("adds");
    let slides = listed(&mut worker, &doc);
    assert!(!has(&slides[0], "title") && !has(&slides[0], "body"), "{:?}", slides[0]);
    assert!(has(&slides[2], "title") && has(&slides[2], "body") && has(&slides[3], "body"), "{slides:?}");
    let before = steps(&mut worker, &doc);
    worker.run(&doc, "slide.text", &["2", "title", "Plan"]).expect("a title");
    worker.run(&doc, "slide.text", &["2", "body", "One\nTwo"]).expect("a body");
    worker.run(&doc, "slide.text", &["3", "body", "Subtitle"]).expect("a subtitle");
    assert_eq!(steps(&mut worker, &doc), before + 3, "one step each");
    assert_eq!(doc.sheets(&mut worker).expect("sheets").current, 0, "the view stays");
    assert_eq!(listed(&mut worker, &doc)[2].title, "Plan");
    let out = dir.path().join("set.fodp");
    doc.save_as(&mut worker, &crate::file_url(&out), "fodp").expect("saves");
    let xml = std::fs::read_to_string(&out).expect("reads");
    for text in ["Plan", "One", "Two", "Subtitle"] {
        assert!(xml.contains(&format!(">{text}</text:p>")), "{text} is its own paragraph");
    }
    worker.run(&doc, "slide.text", &["2", "title", ""]).expect("clears");
    assert_eq!(listed(&mut worker, &doc)[2].title, "", "back to the placeholder");
    worker.run(&doc, "edit.undo", &[]).expect("undoes");
    assert_eq!(listed(&mut worker, &doc)[2].title, "Plan");
    for (values, why) in [
        (["0", "title", "x"], "no placeholder"),
        (["2", "notes", "x"], "notes go through notes.set"),
        (["9", "title", "x"], "no such slide"),
        (["2", "side", "x"], "no such box"),
    ] {
        worker.run(&doc, "slide.text", &values).expect_err(why);
    }
    // Notes of a slide not shown, listed by index.
    let at = worker.now(&doc).expect("the state");
    worker.run(&at, "notes.set", &["Say hi\nthen go", "2"]).expect("notes");
    let slides = listed(&mut worker, &doc);
    assert_eq!((slides[2].notes.as_str(), slides[0].notes.as_str()), ("Say hi\nthen go", ""));
    assert!(slides[2].boxes.contains(&("notes".into(), false)), "{:?}", slides[2]);
}

#[test]
fn a_title_of_mixed_looks_is_refused() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_deck();
    worker.run(&doc, "slide.at", &["add", "2", "1"]).expect("adds");
    worker.run(&doc, "slide.text", &["2", "title", "Big news"]).expect("a title");
    let plain = dir.path().join("plain.fodp");
    doc.save_as(&mut worker, &crate::file_url(&plain), "fodp").expect("saves");
    // Make one word bold, as another editor would.
    let bold = r#"<style:style style:name="Bold" style:family="text"><style:text-properties fo:font-weight="bold"/></style:style>"#;
    let xml = std::fs::read_to_string(&plain).expect("reads");
    assert!(xml.contains(">Big news</text:p>"), "the title on disk");
    let xml = xml
        .replacen("<office:automatic-styles>", &format!("<office:automatic-styles>{bold}"), 1)
        .replacen(">Big news</text:p>", r#">Big <text:span text:style-name="Bold">news</text:span></text:p>"#, 1);
    let rich = dir.path().join("rich.fodp");
    std::fs::write(&rich, xml).expect("writes");
    let doc = worker.load(&crate::file_url(&rich)).expect("loads");
    let slides = listed(&mut worker, &doc);
    assert_eq!(slides[2].title, "Big news");
    assert!(slides[2].boxes.contains(&("title".into(), true)), "{:?}", slides[2]);
    worker.run(&doc, "slide.text", &["2", "title", "Plain"]).expect_err("refuses");
    assert_eq!(listed(&mut worker, &doc)[2].title, "Big news", "not overwritten");
}
