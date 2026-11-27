//! #147: one numbered item restarts, continues or takes its own style.

use super::input::open;
use super::*;

const STYLES: &str = r#"<text:list-style style:name="L1"><text:list-level-style-number text:level="1" style:num-format="1" style:num-suffix="."/><text:list-level-style-number text:level="2" style:num-format="1" text:display-levels="2" style:num-suffix="."/></text:list-style><text:list-style style:name="L2"><text:list-level-style-number text:level="1" style:num-format="1" style:num-suffix="."/></text:list-style>"#;

/// Clause 13 with three items, then two lists split by a paragraph.
fn body() -> String {
    let item = |t: &str| format!("<text:list-item><text:p>{t}</text:p></text:list-item>");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text"><office:automatic-styles>{STYLES}</office:automatic-styles><office:body><office:text><text:list text:style-name="L1"><text:list-item text:start-value="13"><text:p>Thirteen</text:p><text:list>{}{}{}</text:list></text:list-item></text:list><text:p>plain</text:p><text:list text:style-name="L2">{}{}{}</text:list><text:p>gap</text:p><text:list text:style-name="L2">{}{}</text:list></office:text></office:body></office:document>"#,
        item("a"), item("b"), item("c"), item("one"), item("two"), item("three"), item("four"), item("five")
    )
}

const DOWN: i32 = 1024;
const HOME: i32 = 1028;
const MOD1: i32 = 8192;
const NUMBER: &str = ".uno:ElyNumberFrom";
const JOIN: &str = ".uno:ElyNumberJoin";

/// Reported states, kept: the kernel reports changes only.
struct Seen {
    events: Events,
    states: std::collections::HashMap<String, String>,
}

impl Seen {
    /// Polls until `done` holds or `wait` runs out.
    fn poll(&mut self, worker: &mut Worker, doc: &Doc, wait: std::time::Duration, done: impl Fn(&Self) -> bool) {
        let deadline = std::time::Instant::now() + wait;
        loop {
            // One batch of events at a time.
            self.events = poll_until(worker, doc, &self.events, std::time::Duration::from_millis(100), |_| true);
            self.states.extend(self.events.states.iter().cloned());
            if done(self) || std::time::Instant::now() > deadline {
                return;
            }
        }
    }

    fn state(&self, name: &str) -> Option<&str> {
        self.states.get(name).map(String::as_str)
    }

    /// The cursor on paragraph `line`, counted from the top.
    fn to(&mut self, worker: &mut Worker, doc: &Doc, line: usize) {
        worker.post_key(doc, 0, HOME | MOD1).expect("home");
        for _ in 0..line {
            worker.post_key(doc, 0, DOWN).expect("down");
        }
        // The cursor and its states settle within a few frames.
        self.poll(worker, doc, std::time::Duration::from_millis(800), |_| false);
    }
}

/// Each paragraph's label and text, as plain text shows them.
fn shown(dir: &tempfile::TempDir, worker: &mut Worker, doc: &Doc) -> Vec<String> {
    let path = dir.path().join("shown.txt");
    doc.export_plain(worker, &crate::file_url(&path)).expect("exports");
    let text = std::fs::read_to_string(&path).expect("reads");
    text.trim_start_matches('\u{feff}').lines().map(|line| line.trim().to_string()).collect()
}

#[test]
fn one_item_restarts_continues_and_takes_its_own_style() {
    if install().is_none() {
        return eprintln!("SKIP: no pinned LOK build");
    }
    let (dir, mut worker, doc) = open(&body());
    let mut seen = Seen { events: Events::default(), states: Default::default() };
    seen.poll(&mut worker, &doc, HANG, |seen| seen.events.cursor.is_some());
    seen.to(&mut worker, &doc, 2);
    assert_eq!(seen.state(NUMBER), Some("2"), "13.2 reads its own number");
    // 13.2 set to 5: 13.1, 13.5, 13.6, and 13 stays.
    worker.run(&doc, "list.number_from", &["5"]).expect("sets 5");
    seen.poll(&mut worker, &doc, HANG, |seen| seen.state(NUMBER) == Some("5"));
    assert_eq!(shown(&dir, &mut worker, &doc)[..4], ["13. Thirteen", "13.1. a", "13.5. b", "13.6. c"]);
    seen.to(&mut worker, &doc, 4);
    assert_eq!(seen.state(NUMBER), Some(""), "a plain paragraph has no number");
    // "three" restarts; "four" then continues it, its list along.
    seen.to(&mut worker, &doc, 7);
    worker.run(&doc, "list.number_from", &["1"]).expect("restarts");
    seen.poll(&mut worker, &doc, HANG, |seen| seen.state(NUMBER) == Some("1"));
    seen.to(&mut worker, &doc, 9);
    assert_eq!(seen.state(JOIN), Some("enabled"), "a first item after a list");
    worker.run(&doc, "list.join", &[]).expect("continues");
    seen.poll(&mut worker, &doc, HANG, |seen| seen.state(NUMBER) == Some("2"));
    assert_eq!(shown(&dir, &mut worker, &doc)[5..], ["1. one", "2. two", "1. three", "gap", "2. four", "3. five"]);
    // Continuing a restart takes the restart away.
    seen.to(&mut worker, &doc, 7);
    worker.run(&doc, "list.join", &[]).expect("continues");
    seen.poll(&mut worker, &doc, HANG, |seen| seen.state(NUMBER) == Some("3"));
    assert_eq!(seen.state(JOIN), Some("disabled"), "nothing left to continue");
    // "two" alone becomes (2); the list counts on.
    seen.to(&mut worker, &doc, 6);
    worker.run(&doc, "list.number_format", &["4|(#)"]).expect("styles");
    seen.to(&mut worker, &doc, 3);
    worker.run(&doc, "list.number_format", &["10|#)"]).expect("styles a sub-item");
    let styled = ["13.f) c", "plain", "1. one", "(2) two", "3. three", "gap", "4. four", "5. five"];
    assert_eq!(shown(&dir, &mut worker, &doc)[3..], styled);
    assert!(worker.run(&doc, "list.number_format", &["4|x"]).is_err(), "a pattern without # changes nothing");
    // The DOCX keeps every number when read back.
    let path = dir.path().join("numbered.docx");
    doc.save_as(&mut worker, &crate::file_url(&path), "docx").expect("saves");
    let back = worker.load(&crate::file_url(&path)).expect("reloads");
    let _ = poll_until(&mut worker, &back, &Events::default(), HANG, |seen| seen.cursor.is_some());
    assert_eq!(shown(&dir, &mut worker, &back)[2..], [&["13.5. b"][..], &styled[..]].concat());
}
