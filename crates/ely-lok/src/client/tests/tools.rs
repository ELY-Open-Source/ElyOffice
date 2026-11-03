use super::input::{open, saved};
use super::*;
use crate::{menu, review};

const TEXT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>The cat saw a Cat and a dog. Teh cat ran wiht cats.</text:p></office:text></office:body>
</office:document>"#;

/// Runs a search and returns its matches.
fn search(worker: &mut Worker, doc: &Doc, seen: &mut Events, id: &str, values: &[&str]) {
    let before = seen.searched;
    worker
        .run(doc, id, values)
        .unwrap_or_else(|error| panic!("{id}: {error}"));
    *seen = poll_until(worker, doc, seen, HANG, |seen| seen.searched > before);
    assert!(seen.searched > before, "{id} reports an outcome");
}

/// Right-clicks at a point and returns the kernel's menu.
fn menu(worker: &mut Worker, doc: &Doc, seen: &mut Events, (x, y): (i32, i32)) -> Option<String> {
    click(worker, doc, x, y);
    for kind in [0, 1] {
        let event = Mouse {
            kind,
            x,
            y,
            count: 1,
            buttons: 4,
            modifiers: 0,
        };
        worker.post_mouse(doc, event).expect("the click posts");
    }
    *seen = poll_until(worker, doc, seen, HANG, |seen| seen.menu.is_some());
    seen.menu.take()
}

#[test]
fn find_counts_steps_and_replaces() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(TEXT);
    click(&mut worker, &doc, 150, 150);
    let mut seen = Events::default();
    let find = |all: &'static str, case: &'static str, word: &'static str| {
        ["cat", "false", all, case, word, "1"]
    };
    search(
        &mut worker,
        &doc,
        &mut seen,
        "find.run",
        &find("1", "256", "0"),
    );
    assert_eq!(seen.found.len(), 4, "any case, inside words too");
    search(
        &mut worker,
        &doc,
        &mut seen,
        "find.run",
        &find("1", "0", "0"),
    );
    assert_eq!(seen.found.len(), 3, "match case");
    search(
        &mut worker,
        &doc,
        &mut seen,
        "find.run",
        &find("1", "256", "16"),
    );
    assert_eq!(seen.found.len(), 3, "whole words");
    let all = seen.found.clone();
    search(
        &mut worker,
        &doc,
        &mut seen,
        "find.run",
        &find("0", "256", "16"),
    );
    assert_eq!(seen.found, all[..1], "next selects the first");
    search(
        &mut worker,
        &doc,
        &mut seen,
        "find.run",
        &["zebra", "false", "0", "256", "0", "1"],
    );
    assert!(
        seen.found.is_empty(),
        "no match is an outcome, not an error"
    );
    let replace = ["Cat", "false", "3", "0", "16", "1", "Lion"];
    worker
        .run(&doc, "find.replace", &replace)
        .expect("replace all");
    let counts = review::counts(&worker.values(&doc, ".uno:WordCount").expect("counts"))
        .expect("counts parse");
    assert_eq!((counts.words, counts.selected_words), (13, 0));
    let text = saved(&dir, &mut worker, &doc, "found.fodt");
    assert!(
        text.contains("The cat saw a Lion and a dog."),
        "only the cased whole word goes"
    );
}

#[test]
fn spelling_suggests_and_ignores() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(TEXT);
    click(&mut worker, &doc, 150, 150);
    let mut seen = Events::default();
    search(
        &mut worker,
        &doc,
        &mut seen,
        "find.run",
        &["Teh", "false", "0", "0", "0", "1"],
    );
    let (x, y, _, h) = seen.found[0];
    // Writer's idle spell check marks the word later.
    let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let spelling = loop {
        let json = menu(&mut worker, &doc, &mut seen, (x + 60, y + h / 2)).expect("a menu");
        let spelling = menu::read(&json).expect("the menu parses").spelling;
        if spelling.is_some() || std::time::Instant::now() > until {
            break spelling.expect("a spelling menu");
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    let (_, rule) = spelling
        .suggestions
        .iter()
        .find(|(label, _)| label == "The")
        .expect("The is offered");
    worker
        .run(&doc, "spell.apply", &[rule])
        .expect("the suggestion applies");
    search(
        &mut worker,
        &doc,
        &mut seen,
        "find.run",
        &["wiht", "false", "0", "0", "0", "1"],
    );
    let (x, y, _, h) = seen.found[0];
    let json = menu(&mut worker, &doc, &mut seen, (x + 60, y + h / 2)).expect("a menu");
    assert!(menu::read(&json).expect("parses").spelling.is_some());
    worker
        .run(&doc, "spell.ignore_all", &["Spelling"])
        .expect("ignore all");
    let json = menu(&mut worker, &doc, &mut seen, (x + 60, y + h / 2)).expect("a menu");
    assert_eq!(
        menu::read(&json).expect("parses").spelling,
        None,
        "an ignored word is no longer wrong"
    );
    let text = saved(&dir, &mut worker, &doc, "spelled.fodt");
    assert!(text.contains("The cat ran wiht"), "the suggestion lands");
}

#[test]
fn compare_marks_changes_against_an_older_file() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(TEXT);
    let older = dir.path().join("older.fodt");
    let before = TEXT.replace("a dog", "a bird");
    std::fs::write(&older, &before).expect("the older file writes");
    worker
        .run(&doc, "doc.compare", &[&crate::file_url(&older), "true"])
        .expect("the compare runs");
    let changes = review::changes(
        &worker
            .values(&doc, ".uno:AcceptTrackedChanges")
            .expect("changes"),
    )
    .expect("changes parse");
    assert!(
        changes
            .iter()
            .any(|change| change.description.contains("dog")),
        "the newer words show as changes: {changes:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&older).expect("reads"),
        before,
        "the older file stays"
    );
}
