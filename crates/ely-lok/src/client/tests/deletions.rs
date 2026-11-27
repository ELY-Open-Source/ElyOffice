//! #143: tracked deletions leave the line at once; reported.

use super::input::open;
use super::*;
use crate::review;

const BODY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>甲   乙丙</text:p></office:text></office:body>
</office:document>"#;

const HIDE: &str = ".uno:HideTrackedDeletions";
const SHOWN: &str = ".uno:ShowTrackedChanges";
const HOME: i32 = 1028;
const END: i32 = 1029;
const RIGHT: i32 = 1027;
const DELETE: i32 = 1286;

/// Posts `keys`; where the cursor rests, and what was seen.
fn after(worker: &mut Worker, doc: &Doc, from: &Events, keys: &[i32]) -> (i32, Events) {
    for key in keys {
        worker.post_key(doc, 0, *key).expect("a key");
    }
    // The cursor settles within a few frames.
    let seen = poll_until(worker, doc, from, std::time::Duration::from_millis(800), |_| false);
    (seen.cursor.expect("a cursor").0, seen)
}

fn state<'a>(seen: &'a Events, name: &str) -> Option<&'a str> {
    seen.states.iter().rev().find(|(said, _)| said == name).map(|(_, value)| value.as_str())
}

/// Runs `id` and waits for `name` to report `want`.
fn set(worker: &mut Worker, doc: &Doc, from: &Events, (id, name, want): (&str, &str, &str)) -> Events {
    worker.run(doc, id, &[want]).expect(id);
    poll_until(worker, doc, from, HANG, |seen| state(seen, name) == Some(want))
}

fn changes(worker: &mut Worker, doc: &Doc) -> usize {
    let json = worker.values(doc, ".uno:AcceptTrackedChanges").expect("the changes read");
    review::changes(&json).expect("the changes parse").len()
}

#[test]
fn tracked_deletions_reflow_unless_struck_in_place() {
    if install().is_none() {
        return eprintln!("SKIP: no pinned LOK build");
    }
    let (dir, mut worker, doc) = open(BODY);
    let loaded = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| state(seen, HIDE).is_some());
    assert_eq!(state(&loaded, HIDE), Some("false"), "the kernel strikes in place");
    let hidden = set(&mut worker, &doc, &loaded, ("track.hide", HIDE, "true"));
    // Reported again only if it changed.
    let shown = state(&hidden, SHOWN).or(state(&loaded, SHOWN));
    assert_eq!(shown, Some("true"), "still Markup, not Final");
    click(&mut worker, &doc, 150, 150);
    worker.run(&doc, "track.toggle", &[]).expect("tracking on");
    let (start, seen) = after(&mut worker, &doc, &hidden, &[END]);
    let (_, seen) = after(&mut worker, &doc, &seen, &[HOME, RIGHT, DELETE, DELETE, DELETE]);
    let (reflowed, seen) = after(&mut worker, &doc, &seen, &[END]);
    assert!(reflowed < start, "the line closes up: {reflowed} < {start}");
    assert_eq!(changes(&mut worker, &doc), 1, "one deletion, listed");
    // A save keeps the file in Markup.
    let saved = dir.path().join("saved.fodt");
    doc.save_as(&mut worker, &crate::file_url(&saved), "fodt").expect("saves");
    let xml = std::fs::read_to_string(&saved).expect("the save reads");
    let shown = r#"config:name="ShowRedlineChanges" config:type="boolean">true<"#;
    assert!(xml.contains(shown), "saved as shown changes");
    // Final and back keeps deletions out.
    let fin = set(&mut worker, &doc, &seen, ("track.shown", SHOWN, "false"));
    let back = set(&mut worker, &doc, &fin, ("track.shown", SHOWN, "true"));
    assert_ne!(state(&back, HIDE), Some("false"), "Final keeps the choice");
    let (marked, seen) = after(&mut worker, &doc, &back, &[HOME, END]);
    assert_eq!(marked, reflowed, "out of the text after Final");
    // Struck in place, the spaces keep their width.
    let struck = set(&mut worker, &doc, &seen, ("track.hide", HIDE, "false"));
    let (kept, seen) = after(&mut worker, &doc, &struck, &[HOME, END]);
    assert!(kept > reflowed, "the gap is back: {kept} > {reflowed}");
    let out = set(&mut worker, &doc, &seen, ("track.hide", HIDE, "true"));
    let (again, seen) = after(&mut worker, &doc, &out, &[HOME, END]);
    assert_eq!(again, reflowed, "out of the text again");
    // Rejecting brings the spaces back.
    worker.run(&doc, "track.reject_all", &[]).expect("rejected");
    let (back, _) = after(&mut worker, &doc, &seen, &[HOME, END]);
    assert_eq!(changes(&mut worker, &doc), 0);
    assert_eq!(back, start, "the spaces return");
}
