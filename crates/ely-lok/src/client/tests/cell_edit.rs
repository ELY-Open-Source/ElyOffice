//! Saves keep an open cell edit (patch 0082).

use super::sheet::open_sheet;
use super::*;
use crate::sheet;

/// Longest wait for a key or a state.
const SETTLE: std::time::Duration = std::time::Duration::from_secs(3);

/// F2 on A1, then Z as a key or text.
fn editing(worker: &mut Worker, doc: &Doc, text: bool) -> Events {
    let base = poll_until(worker, doc, &Events::default(), SETTLE, |_| false);
    worker.post_key(doc, 0, 769).expect("F2 edits A1");
    match text {
        true => {
            worker.post_text(doc, false, "Z").expect("Z types");
            worker.post_text(doc, true, "").expect("Z commits");
        }
        false => worker.post_key(doc, 'Z' as u32, 537 | 0x1000).expect("Z types"),
    }
    let typed = poll_until(worker, doc, &base, SETTLE, |seen| {
        seen.a11y.text == "Hello 世界Z" && modified(seen) == Some("true")
    });
    assert_eq!(typed.a11y.text, "Hello 世界Z", "the edit is open");
    assert_eq!(modified(&typed), Some("true"), "text {text}: typing marks the document modified");
    typed
}

/// The last modified state reported in `seen`.
fn modified(seen: &Events) -> Option<&str> {
    seen.states.iter().rev().find(|(name, _)| name == ".uno:ModifiedStatus").map(|(_, value)| value.as_str())
}

/// A1 as entered, after Enter ends the edit.
fn entered(worker: &mut Worker, doc: &Doc, from: &Events) -> String {
    worker.post_key(doc, 0, 1280).expect("Enter enters");
    poll_until(worker, doc, from, SETTLE, |seen| seen.a11y.text.is_empty());
    let read = worker.values(doc, &sheet::cell_at_query("S!A1")).expect("reads A1");
    sheet::cell_at(&read).expect("A1").text
}

#[test]
fn a_save_keeps_the_cell_edit_and_writes_the_cell_as_entered() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    for (owned, text) in [(false, false), (true, false), (false, true), (true, true)] {
        let (dir, mut worker, doc) = open_sheet();
        let typed = editing(&mut worker, &doc, text);
        let out = dir.path().join("saved.fods");
        let url = crate::file_url(&out);
        match owned {
            true => doc.save_owned(&mut worker, &url, "fods"),
            false => doc.save_as(&mut worker, &url, "fods"),
        }
        .expect("saves");
        let after = poll_until(&mut worker, &doc, &typed, SETTLE, |seen| modified(seen) == Some("false"));
        assert_ne!(modified(&after), Some("false"), "owned {owned} text {text}: the typed text stays unsaved");
        worker.post_key(&doc, 'Y' as u32, 536 | 0x1000).expect("Y types");
        let more = poll_until(&mut worker, &doc, &after, SETTLE, |seen| seen.a11y.text.ends_with('Y'));
        assert_eq!(more.a11y.text, "Hello 世界ZY", "owned {owned} text {text}: the keys go on in the edit");
        let xml = std::fs::read_to_string(&out).expect("the saved file");
        assert!(xml.contains("Hello 世界</text:p>"), "owned {owned} text {text}: the cell as entered");
        assert!(!xml.contains("Hello 世界Z"), "owned {owned} text {text}: the open edit is not written");
        assert_eq!(entered(&mut worker, &doc, &more), "Hello 世界ZY", "owned {owned} text {text}");
    }
}

#[test]
fn a_closing_save_enters_the_cell_edit_first() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_sheet();
    editing(&mut worker, &doc, true);
    let out = dir.path().join("closing.fods");
    doc.save_closing(&mut worker, &crate::file_url(&out), "fods").expect("saves");
    let xml = std::fs::read_to_string(&out).expect("the saved file");
    assert!(xml.contains("Hello 世界Z</text:p>"), "the typed text is written");
}

#[test]
fn a_closing_save_refuses_an_entry_its_rule_refuses() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_sheet();
    let rule = sheet::Validation {
        kind: "list".into(),
        operator: "equal".into(),
        list: vec!["Red".into()],
        show_error: true,
        error_style: "stop".into(),
        ..sheet::Validation::default()
    };
    worker.run(&doc, "rules.validation", &[&rule.to_json()]).expect("the rule");
    let ruled = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
    // The rule already marked the document modified.
    worker.post_key(&doc, 0, 769).expect("F2 edits A1");
    worker.post_text(&doc, false, "Z").expect("Z types");
    worker.post_text(&doc, true, "").expect("Z commits");
    let typed = poll_until(&mut worker, &doc, &ruled, SETTLE, |seen| seen.a11y.text == "Hello 世界Z");
    assert_eq!(typed.a11y.text, "Hello 世界Z", "the edit is open");
    let out = dir.path().join("closing.fods");
    let refused = doc.save_closing(&mut worker, &crate::file_url(&out), "fods").expect_err("the entry is refused");
    assert!(refused.to_string().contains("not accepted"), "the kernel says why: {refused}");
    assert!(!out.exists(), "nothing is written");
    worker.post_key(&doc, 'Y' as u32, 536 | 0x1000).expect("Y types");
    let more = poll_until(&mut worker, &doc, &typed, SETTLE, |seen| seen.a11y.text.ends_with('Y'));
    assert_eq!(more.a11y.text, "Hello 世界ZY", "the edit stays open to fix");
}

#[test]
fn a_cancelled_composition_leaves_the_cell_as_it_was() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_sheet();
    let base = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
    worker.post_text(&doc, false, "n").expect("a composition");
    let composing = poll_until(&mut worker, &doc, &base, SETTLE, |seen| seen.a11y.text == "n");
    assert_eq!(composing.a11y.text, "n", "the composition opens an edit");
    worker.post_text(&doc, false, "").expect("the cancel");
    worker.post_text(&doc, true, "").expect("the end");
    let back = poll_until(&mut worker, &doc, &composing, SETTLE, |seen| {
        seen.a11y.text == "Hello 世界" && modified(seen) == Some("false")
    });
    assert_eq!(back.a11y.text, "Hello 世界", "the cell as it was");
    assert_eq!(modified(&back), Some("false"), "nothing changed");
    let out = dir.path().join("closing.fods");
    doc.save_closing(&mut worker, &crate::file_url(&out), "fods").expect("saves");
    let xml = std::fs::read_to_string(&out).expect("the saved file");
    assert!(xml.contains("Hello 世界</text:p>"), "the cell is kept");
}
