//! The first-line indent counts CJK characters (#76).

use super::input::{open, saved};
use super::*;

/// Two paragraphs of the same Chinese text.
const TWICE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>中文排版首行缩进两个字符</text:p><text:p>中文排版首行缩进两个字符</text:p></office:text></office:body>
</office:document>"#;

const STATE: &str = ".uno:ElyFirstLineChars";
const MOD1: i32 = 0x2000;
const HOME: i32 = 1028;
const DOWN: i32 = 1024;
const RIGHT: i32 = 1027;

fn state<'a>(seen: &'a Events, uno: &str) -> Option<&'a str> {
    seen.states.iter().rev().find(|(name, _)| name == uno).map(|(_, value)| value.as_str())
}

/// Posts `keys`; the resting cursor's edge and states seen.
fn cursor_after(worker: &mut Worker, doc: &Doc, keys: &[i32]) -> (i32, Events) {
    let before = poll_until(worker, doc, &Events::default(), std::time::Duration::from_millis(300), |_| false);
    for key in keys {
        worker.post_key(doc, 0, *key).expect("a key");
    }
    let mut seen = poll_until(worker, doc, &before, std::time::Duration::from_millis(600), |_| false);
    // States come once; keep the earlier poll's.
    seen.states.splice(0..0, before.states);
    (seen.cursor.expect("a cursor").0, seen)
}

#[test]
fn two_characters_indent_the_first_line_by_two_of_its_own() {
    if install().is_none() {
        return eprintln!("SKIP: no pinned LOK build");
    }
    let (dir, mut worker, doc) = open(TWICE);
    click(&mut worker, &doc, 150, 150);
    let (start, early) = cursor_after(&mut worker, &doc, &[MOD1 | HOME]);
    let none = match state(&early, STATE) {
        Some(_) => early,
        None => poll_until(&mut worker, &doc, &early, HANG, |seen| state(seen, STATE).is_some()),
    };
    assert_eq!(state(&none, STATE), Some("0,twip"), "a plain paragraph has no indent");
    worker.run(&doc, "paragraph.first_line", &["200"]).expect("two characters");
    let set = poll_until(&mut worker, &doc, &none, HANG, |seen| state(seen, STATE) == Some("200,ic"));
    assert_eq!(state(&set, STATE), Some("200,ic"));
    let (indented, _) = cursor_after(&mut worker, &doc, &[MOD1 | HOME]);
    // The second paragraph, unindented: two characters in.
    let (second, _) = cursor_after(&mut worker, &doc, &[DOWN, HOME, RIGHT, RIGHT]);
    assert!(indented > start, "the first line moved in");
    assert_eq!(indented, second, "the indent is two of the line's own characters");
    // DOCX keeps it as characters, not as a length.
    let docx = dir.path().join("indent.docx");
    doc.save_as(&mut worker, &crate::file_url(&docx), "docx").expect("saves");
    let back = worker.load(&crate::file_url(&docx)).expect("reloads");
    click(&mut worker, &back, 150, 150);
    worker.post_key(&back, 0, MOD1 | HOME).expect("to the start");
    let read = poll_until(&mut worker, &back, &Events::default(), HANG, |seen| state(seen, STATE) == Some("200,ic"));
    assert_eq!(state(&read, STATE), Some("200,ic"), "DOCX brought back another indent");
    let flat = saved(&dir, &mut worker, &doc, "indent.fodt");
    assert!(flat.contains(r#"loext:text-indent="2ic""#), "ODF writes the unit");
}

#[test]
fn a_value_sent_again_after_a_mixed_state_arrives() {
    if install().is_none() {
        return eprintln!("SKIP: no pinned LOK build");
    }
    let (_dir, mut worker, doc) = open(TWICE);
    click(&mut worker, &doc, 150, 150);
    cursor_after(&mut worker, &doc, &[MOD1 | HOME]);
    let mut seen = Events::default();
    let mut reach = |worker: &mut Worker, id: &str, values: &[&str], want: &str| {
        worker.run(&doc, id, values).expect("the command runs");
        seen = poll_until(worker, &doc, &seen, HANG, |seen| state(seen, STATE) == Some(want));
        assert_eq!(state(&seen, STATE), Some(want), "after {id}");
    };
    reach(&mut worker, "paragraph.first_line", &["200"], "200,ic");
    reach(&mut worker, "edit.select_all", &[], crate::MIXED);
    reach(&mut worker, "paragraph.first_line", &["-150"], "-150,ic");
    reach(&mut worker, "edit.undo", &[], crate::MIXED);
    // The same value as before the mixed state.
    reach(&mut worker, "edit.redo", &[], "-150,ic");
}
