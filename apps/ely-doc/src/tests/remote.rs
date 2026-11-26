//! Another program reads, replaces and saves the text.

use std::cell::RefCell;

use ely_office::prelude::*;
use gpui::{Entity, TestAppContext};

use crate::docs::Docs;
use crate::review::TrackChanges;
use crate::tests::{blank, docs, settle, settled, text_of, type_into, with_window};

/// The answer to `words`, once it comes.
fn ask(cx: &mut TestAppContext, docs: &Entity<Docs>, words: &[&str]) -> Result<String, String> {
    let (ask, answered) = ely_ui::Ask::for_tests(&serde_json::json!(words).to_string());
    cx.update(|app| docs.update(app, |docs, cx| docs.answer_request(ask, cx)));
    let answer = RefCell::new(None);
    settle(cx, |_| {
        let mut got = answer.borrow_mut();
        if got.is_none() {
            *got = answered.try_recv().ok();
        }
        got.is_some()
    });
    answer.into_inner().expect("answered")
}

#[test]
fn text_reads_replaces_in_one_step_and_saves_with_the_cursor_kept() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "cat Cat 世界 😀 cat");
    let path = dir.path().join("doc.odt");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, path.clone(), None, window, cx));
    settle(&cx, |app| docs.read(app).core.tabs[0].name == "doc.odt");
    let file = path.display().to_string();
    let docs_listed: Vec<String> = serde_json::from_str(&ask(&mut cx, &docs, &["docs"]).expect("docs")).expect("a JSON list");
    let canonical: Vec<_> = docs_listed.iter().map(|file| std::fs::canonicalize(file).ok()).collect();
    assert_eq!(canonical, [std::fs::canonicalize(&path).ok()], "{docs_listed:?}");
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("cat Cat 世界 😀 cat".into()));
    assert_eq!(ask(&mut cx, &docs, &["replace", &file, "cat", "dog"]), Ok(String::new()));
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("dog Cat 世界 😀 dog".into()));
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("dog Cat 世界 😀 dog".into()));
    settled(&cx, &docs);
    cx.update(|app| {
        let tab = &docs.read(app).core.tabs[0];
        assert!(tab.dirty(app), "a read leaves the edit unsaved");
        // macOS keeps temp files behind /var's link.
        let kept = tab.session.path().and_then(|kept| std::fs::canonicalize(kept).ok());
        assert_eq!(kept, std::fs::canonicalize(&path).ok(), "a read keeps the file");
    });
    let missing = ask(&mut cx, &docs, &["replace", &file, "zebra", "x"]);
    assert!(missing.as_ref().is_err_and(|why| why.starts_with("nothing replaced")), "{missing:?}");
    assert!(ask(&mut cx, &docs, &["replace", &file, "dog\nx", "a"]).is_err(), "a break in FIND");
    let other = dir.path().join("other.odt");
    std::fs::write(&other, b"x").expect("a stray file");
    assert!(ask(&mut cx, &docs, &["get", &other.display().to_string()]).is_err(), "a file not open");
    // Typing lands where the cursor was, not on a match.
    type_into(&mut cx, &docs, "!");
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("dog Cat 世界 😀 dog!".into()));
    cx.update(|app| docs.update(app, |docs, cx| docs.run_active("edit.undo", &[], cx)));
    settled(&cx, &docs);
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("dog Cat 世界 😀 dog".into()));
    cx.update(|app| docs.update(app, |docs, cx| docs.run_active("edit.undo", &[], cx)));
    settled(&cx, &docs);
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("cat Cat 世界 😀 cat".into()), "one undo step");
    cx.update(|app| docs.update(app, |docs, cx| docs.run_active("edit.redo", &[], cx)));
    settled(&cx, &docs);
    let saved: String = serde_json::from_str(&ask(&mut cx, &docs, &["save", &file]).expect("saved")).expect("a JSON path");
    assert_eq!(std::fs::canonicalize(&saved).ok(), std::fs::canonicalize(&path).ok());
    assert!(text_of(&mut cx, &docs, &path).contains("dog Cat 世界 😀 dog"), "on disk at the answer");
    assert_eq!(ask(&mut cx, &docs, &["replace", &file, " ", "_"]), Ok(String::new()));
    assert_eq!(ask(&mut cx, &docs, &["replace", &file, "_", "\t"]), Ok(String::new()));
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("dog\tCat\t世界\t😀\tdog".into()));
    assert_eq!(ask(&mut cx, &docs, &["replace", &file, "\t", " "]), Ok(String::new()));
    let second = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "second");
    let two = dir.path().join("two.odt");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(second, two.clone(), None, window, cx));
    settle(&cx, |app| docs.read(app).core.tabs[1].name == "two.odt");
    assert_eq!(ask(&mut cx, &docs, &["get", &two.display().to_string()]), Ok("second".into()));
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("dog Cat 世界 😀 dog".into()), "each open document reads");
}

#[test]
fn append_and_replace_with_breaks_are_one_step_each_and_keep_the_cursor() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "first");
    let path = dir.path().join("lines.odt");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, path.clone(), None, window, cx));
    settle(&cx, |app| docs.read(app).core.tabs[0].name == "lines.odt");
    let file = path.display().to_string();
    let undo = |cx: &mut TestAppContext| {
        cx.update(|app| docs.update(app, |docs, cx| docs.run_active("edit.undo", &[], cx)));
        settled(cx, &docs);
    };
    assert_eq!(ask(&mut cx, &docs, &["append", &file, "a\r\nb\n", ""]), Ok(String::new()));
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("first\na\nb".into()));
    // Typing stays in the paragraph the cursor was in.
    type_into(&mut cx, &docs, "!");
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("first!\na\nb".into()));
    undo(&mut cx);
    undo(&mut cx);
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("first".into()), "the append is one step");
    assert_eq!(ask(&mut cx, &docs, &["append", &file, "Title", "Heading 1"]), Ok(String::new()));
    let listed: Vec<serde_json::Value> = serde_json::from_str(&ask(&mut cx, &docs, &["get", &file, "--json"]).expect("json")).expect("a JSON list");
    assert_eq!(listed.len(), 2, "{listed:?}");
    assert_eq!(listed[0]["text"], "first");
    assert_eq!(listed[1], serde_json::json!({ "style": "Heading 1", "text": "Title" }));
    let missing = ask(&mut cx, &docs, &["append", &file, "x", "No Such Style"]);
    assert_eq!(missing, Err("No Such Style: no such paragraph style in this document".into()));
    assert_eq!(ask(&mut cx, &docs, &["append", &file, "\n", ""]), Err("TEXT must not be empty".into()));
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("first\nTitle".into()), "refusals change nothing");
    // Regex characters in either side stay literal.
    let none = ask(&mut cx, &docs, &["replace", &file, "f.rst", "a\nb"]);
    assert!(none.as_ref().is_err_and(|why| why.starts_with("nothing replaced")), "{none:?}");
    assert_eq!(ask(&mut cx, &docs, &["replace", &file, "first", "one (.*)\ntwo & $1 \\x"]), Ok(String::new()));
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("one (.*)\ntwo & $1 \\x\nTitle".into()));
    assert!(ask(&mut cx, &docs, &["replace", &file, "two", "a\\nb\nc"]).is_err(), "a backslash before n");
    undo(&mut cx);
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("first\nTitle".into()), "the replace is one step");
}

#[test]
fn format_styles_every_match_in_one_step_with_the_cursor_kept() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "cat dog cat");
    let path = dir.path().join("looks.odt");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, path.clone(), None, window, cx));
    settle(&cx, |app| docs.read(app).core.tabs[0].name == "looks.odt");
    let file = path.display().to_string();
    let undo = |cx: &mut TestAppContext| {
        cx.update(|app| docs.update(app, |docs, cx| docs.run_active("edit.undo", &[], cx)));
        settled(cx, &docs);
    };
    let fields = "on\t\t\t\t280\tB00020\t";
    assert_eq!(ask(&mut cx, &docs, &["format", &file, "cat", fields]), Ok(String::new()));
    // Typing lands where the cursor was, not on a match.
    type_into(&mut cx, &docs, "!");
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("cat dog cat!".into()));
    undo(&mut cx);
    assert!(ask(&mut cx, &docs, &["save", &file]).is_ok(), "saved");
    let saved = text_of(&mut cx, &docs, &path).to_lowercase();
    assert!(saved.contains("fo:font-weight=\"bold\"") && saved.contains("#b00020") && saved.contains("14pt"), "{saved}");
    assert_eq!(saved.matches("<text:span").count(), 2, "both matches, not the dog: {saved}");
    undo(&mut cx);
    assert!(ask(&mut cx, &docs, &["save", &file]).is_ok(), "saved");
    assert!(!text_of(&mut cx, &docs, &path).to_lowercase().contains("#b00020"), "the format is one step");
    assert_eq!(ask(&mut cx, &docs, &["format", &file, "dog", "\t\t\t\t\t\tHeading 1"]), Ok(String::new()));
    assert!(ask(&mut cx, &docs, &["save", &file]).is_ok(), "saved");
    let styled = ely_office::remote::body_paragraphs(text_of(&mut cx, &docs, &path).as_bytes()).expect("paragraphs");
    assert_eq!(styled, [("Heading 1".to_owned(), "cat dog cat".to_owned())]);
    let missing = ask(&mut cx, &docs, &["format", &file, "dog", "\t\t\t\t\t\tNo Such Style"]);
    assert_eq!(missing, Err("No Such Style: no such paragraph style in this document".into()));
    let none = ask(&mut cx, &docs, &["format", &file, "zebra", "on\t\t\t\t\t\t"]);
    assert_eq!(none, Err(format!("{file}: no match for \"zebra\"")));
}

#[test]
fn a_table_lands_at_the_end_or_after_a_match_in_one_step() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "first");
    let path = dir.path().join("table.odt");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, path.clone(), None, window, cx));
    settle(&cx, |app| docs.read(app).core.tabs[0].name == "table.odt");
    let file = path.display().to_string();
    let undo = |cx: &mut TestAppContext| {
        cx.update(|app| docs.update(app, |docs, cx| docs.run_active("edit.undo", &[], cx)));
        settled(cx, &docs);
    };
    let listed = |cx: &mut TestAppContext| -> Vec<(String, String)> {
        let json: Vec<serde_json::Value> = serde_json::from_str(&ask(cx, &docs, &["get", &file, "--json"]).expect("json")).expect("a JSON list");
        json.iter().map(|p| (p["style"].as_str().unwrap_or_default().to_owned(), p["text"].as_str().unwrap_or_default().to_owned())).collect()
    };
    let texts = |list: &[(String, String)]| list.iter().map(|(_, text)| text.clone()).collect::<Vec<_>>();
    let insert = |cx: &mut TestAppContext, cells: &str, after: &str, header: &str| ask(cx, &docs, &["table.insert", &file, cells, after, header]);
    assert_eq!(insert(&mut cx, "Name\u{1f}Qty\u{1e}Tea\u{1f}3", "", "true"), Ok(String::new()));
    // Typing stays in the paragraph the cursor was in.
    type_into(&mut cx, &docs, "!");
    let list = listed(&mut cx);
    assert_eq!(texts(&list), ["first!", "Name", "Qty", "Tea", "3", ""], "a paragraph follows the table: {list:?}");
    assert_eq!((list[1].0.as_str(), list[3].0.as_str()), ("Table Heading", "Table Contents"), "row 1 heads");
    undo(&mut cx);
    undo(&mut cx);
    assert_eq!(texts(&listed(&mut cx)), ["first"], "the table is one step");
    assert_eq!(ask(&mut cx, &docs, &["append", &file, "last", ""]), Ok(String::new()));
    assert_eq!(insert(&mut cx, "x\u{1f}y\nz", "irs", "false"), Ok(String::new()));
    let list = listed(&mut cx);
    assert_eq!((list.len(), list[1].1.as_str(), list[3].1.as_str()), (4, "x", "last"), "after the match: {list:?}");
    assert_eq!(list[1].0, "Table Contents", "no heading row");
    // A match in a table: after that whole table.
    assert_eq!(insert(&mut cx, "q", "x", "false"), Ok(String::new()));
    let list = listed(&mut cx);
    assert_eq!((list.len(), list[3].1.as_str(), list[4].1.as_str()), (5, "q", "last"), "{list:?}");
    assert!(ask(&mut cx, &docs, &["save", &file]).is_ok(), "saved");
    assert!(text_of(&mut cx, &docs, &path).contains("<text:line-break/>"), "a cell's newline breaks its line");
    let none = insert(&mut cx, "a", "zebra", "false");
    assert_eq!(none, Err(format!("{file}: no match for \"zebra\"")));
    assert_eq!(listed(&mut cx).len(), 5, "a refusal changes nothing");
    // Empty and blank cells are data too.
    assert_eq!(insert(&mut cx, "", "", "false"), Ok(String::new()));
    assert_eq!(listed(&mut cx).len(), 7, "one empty cell, then a paragraph");
    assert_eq!(insert(&mut cx, " ", "last", "false"), Ok(String::new()));
    assert_eq!(listed(&mut cx).len(), 8, "one blank cell after the match");
}

/// Each frame's width and height in centimetres.
fn frames(xml: &str) -> Vec<(f64, f64)> {
    let cm = |value: &str| -> f64 {
        let at = value.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(value.len());
        let number: f64 = value[..at].parse().unwrap_or(f64::NAN);
        number * match &value[at..] {
            "cm" => 1.0,
            "mm" => 0.1,
            "in" => 2.54,
            "pt" => 2.54 / 72.0,
            _ => f64::NAN,
        }
    };
    let attribute = |tag: &str, name: &str| tag.split(&format!("{name}=\"")).nth(1).and_then(|rest| rest.split('"').next()).map(cm).unwrap_or(f64::NAN);
    xml.split("<draw:frame ").skip(1).map(|tag| tag.split('>').next().unwrap_or_default()).map(|tag| (attribute(tag, "svg:width"), attribute(tag, "svg:height"))).collect()
}

#[test]
fn a_picture_lands_at_the_end_or_after_a_match_sized_in_one_step() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "first");
    let path = dir.path().join("image.odt");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, path.clone(), None, window, cx));
    settle(&cx, |app| docs.read(app).core.tabs[0].name == "image.odt");
    let file = path.display().to_string();
    let svg = |name: &str, width: u32| {
        let picture = dir.path().join(name);
        let body = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}pt" height="{}pt"><rect width="100%" height="100%" fill="red"/></svg>"#, width / 2);
        std::fs::write(&picture, body).expect("a picture");
        picture
    };
    let small = svg("small.svg", 200);
    let wide = svg("wide.svg", 2000);
    let insert = |cx: &mut TestAppContext, picture: &std::path::Path, after: &str, width: &str| {
        let shown = picture.display().to_string();
        ask(cx, &docs, &["image.insert", &file, &ely_lok::file_url(picture), after, width, &shown])
    };
    let texts = |cx: &mut TestAppContext| -> Vec<String> {
        let json: Vec<serde_json::Value> = serde_json::from_str(&ask(cx, &docs, &["get", &file, "--json"]).expect("json")).expect("a JSON list");
        json.iter().map(|p| p["text"].as_str().unwrap_or_default().to_owned()).collect()
    };
    let saved = |cx: &mut TestAppContext| {
        assert!(ask(cx, &docs, &["save", &file]).is_ok(), "saved");
        text_of(cx, &docs, &path)
    };
    assert_eq!(insert(&mut cx, &small, "", ""), Ok(String::new()));
    // Typing stays in the paragraph the cursor was in.
    type_into(&mut cx, &docs, "!");
    assert_eq!(texts(&mut cx), ["first!", ""], "the picture has its own paragraph");
    let xml = saved(&mut cx);
    assert!(xml.contains("text:anchor-type=\"as-char\"") && xml.contains("<draw:image"), "embedded as a character");
    let natural = frames(&xml);
    assert_eq!(natural.len(), 1, "{natural:?}");
    // Writer rounds an SVG's size when it reads it.
    assert!((natural[0].0 - 200.0 * 2.54 / 72.0).abs() < 0.1, "its own size: {natural:?}");
    cx.update(|app| docs.update(app, |docs, cx| docs.run_active("edit.undo", &[], cx)));
    settled(&cx, &docs);
    cx.update(|app| docs.update(app, |docs, cx| docs.run_active("edit.undo", &[], cx)));
    settled(&cx, &docs);
    assert_eq!(texts(&mut cx), ["first"], "the picture is one step");
    assert_eq!(ask(&mut cx, &docs, &["append", &file, "last", ""]), Ok(String::new()));
    assert_eq!(insert(&mut cx, &wide, "irs", ""), Ok(String::new()));
    assert_eq!(texts(&mut cx), ["first", "", "last"], "after the match");
    let capped = frames(&saved(&mut cx));
    assert!(capped[0].0 > 10.0 && capped[0].0 < 20.0, "within the text width: {capped:?}");
    assert!((capped[0].1 * 2.0 - capped[0].0).abs() < 0.05, "the ratio kept: {capped:?}");
    assert_eq!(insert(&mut cx, &small, "last", "4535"), Ok(String::new()));
    let sized = frames(&saved(&mut cx));
    assert_eq!(sized.len(), 2, "{sized:?}");
    assert!((sized[1].0 - 8.0).abs() < 0.02 && (sized[1].1 - 4.0).abs() < 0.02, "--width 8cm: {sized:?}");
    let none = insert(&mut cx, &small, "zebra", "");
    assert_eq!(none, Err(format!("{file}: no match for \"zebra\"")));
    let fake = dir.path().join("fake.png");
    std::fs::write(&fake, "not a picture").expect("a file");
    let unread = insert(&mut cx, &fake, "", "");
    assert_eq!(unread, Err(format!("{}: cannot read as an image; nothing was inserted", fake.display())));
    assert_eq!(texts(&mut cx).len(), 4, "refusals change nothing");
}

#[test]
fn a_read_never_gives_way_to_waiting_input() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let file = dir.path().join("read.fodt");
    let flat = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>Read me</text:p></office:text></office:body>
</office:document>"#;
    std::fs::write(&file, flat).expect("the file writes");
    let worker = cx.update(|app| docs.read(app).core.worker.clone());
    let waiter = worker.lock().expect("the worker").waiter();
    let mut held = worker.lock().expect("the worker");
    let doc = held.load(&ely_lok::file_url(&file)).expect("the file opens");
    // A key waits for the worker all through the read.
    let waiting = {
        let worker = worker.clone();
        std::thread::spawn(move || drop(waiter.hold(&worker)))
    };
    // By now the key waits behind the lock.
    std::thread::sleep(std::time::Duration::from_millis(300));
    let read = ely_office::remote::copy::<ely_office::Lok>(&mut held, &doc, &dir.path().join("copy.fodt"), "fodt");
    held.close_doc(doc).expect("closes");
    drop(held);
    waiting.join().expect("the waiting key");
    let read = read.expect("a waiting key never stops a read");
    assert_eq!(ely_office::remote::body_text(&read).expect("a body").trim(), "Read me");
}

#[test]
fn replace_leaves_tracked_deletions_alone() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Centred now");
    let path = dir.path().join("tracked.docx");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, path.clone(), None, window, cx));
    settle(&cx, |app| docs.read(app).core.tabs[0].name == "tracked.docx");
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_track(&TrackChanges, window, cx));
    settle(&cx, |app| {
        let tab = docs.read(app).active_tab().expect("a tab");
        tab.view.read(app).state(".uno:TrackChanges") == Some("true")
    });
    let file = path.display().to_string();
    assert_eq!(ask(&mut cx, &docs, &["replace", &file, "Centred", "Centred one"]), Ok(String::new()));
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("Centred one now".into()));
    // The first "Centred" is now deleted text; it stays deleted.
    assert_eq!(ask(&mut cx, &docs, &["replace", &file, "Centred", "Centred two"]), Ok(String::new()));
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("Centred two one now".into()));
    assert_eq!(ask(&mut cx, &docs, &["replace", &file, "now", "then"]), Ok(String::new()));
    let gone = ask(&mut cx, &docs, &["replace", &file, "now", "later"]);
    assert!(gone.as_ref().is_err_and(|why| why.starts_with("nothing replaced")), "only deleted: {gone:?}");
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("Centred two one then".into()));
    assert_eq!(ask(&mut cx, &docs, &["save", &file]).map(|_| ()), Ok(()));
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("Centred two one then".into()), "a read after saving");
}
