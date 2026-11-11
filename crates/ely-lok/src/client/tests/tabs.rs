//! A tab stop is removed only where it was shown.

use super::input::{open, saved};
use super::*;

const TABS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:automatic-styles>
  <style:style style:name="P1" style:family="paragraph"><style:paragraph-properties><style:tab-stops><style:tab-stop style:position="0.5in"/><style:tab-stop style:position="1in"/></style:tab-stops></style:paragraph-properties></style:style>
  <style:style style:name="P2" style:family="paragraph"><style:paragraph-properties><style:tab-stops><style:tab-stop style:position="1.5in"/><style:tab-stop style:position="2in"/></style:tab-stops></style:paragraph-properties></style:style>
 </office:automatic-styles>
 <office:body><office:text><text:p text:style-name="P1">one</text:p><text:p text:style-name="P2">two</text:p></office:text></office:body>
</office:document>"#;

fn stops(xml: &str) -> usize {
    xml.matches("<style:tab-stop ").count()
}

#[test]
fn a_stop_shown_for_the_last_paragraph_removes_none_in_the_next() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(TABS);
    worker.post_key(&doc, 0, 1027).expect("Right posts");
    let shown = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| seen.tabs == [1270, 2540]);
    let top = shown.cursor.expect("a cursor").1;
    worker.post_key(&doc, 0, 1024).expect("Down posts");
    // The ruler lags the cursor: P1's stops still show.
    let moved = poll_until(&mut worker, &doc, &shown, HANG, |seen| seen.cursor.is_some_and(|at| at.1 > top));
    let read = worker.now(&doc).expect("the revision");
    let refused = worker.run(&read, "paragraph.untab", &["1", "2540"]);
    assert!(refused.is_err_and(|error| !crate::is_stale(&error)), "the kernel kept P2's stop");
    poll_until(&mut worker, &doc, &moved, HANG, |seen| seen.tabs == [3810, 5080]);
    assert_eq!(stops(&saved(&dir, &mut worker, &doc, "kept.fodt")), 4, "both paragraphs keep theirs");
    let read = worker.now(&doc).expect("the revision");
    worker.run(&read, "paragraph.untab", &["1", "5080"]).expect("the shown stop goes");
    assert_eq!(stops(&saved(&dir, &mut worker, &doc, "out.fodt")), 3, "one stop went");
}

#[test]
fn a_stop_left_of_the_page_edge_is_removed_too() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    // A negative indent puts the ruler's stops below zero.
    let body = TABS.replace(r#"style:family="paragraph"><style:paragraph-properties>"#, r#"style:family="paragraph"><style:paragraph-properties fo:margin-left="-1in">"#);
    let (dir, mut worker, doc) = open(&body);
    worker.post_key(&doc, 0, 1027).expect("Right posts");
    let shown = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| seen.tabs.first().is_some_and(|at| *at < 0));
    let at = shown.tabs[0].to_string();
    let read = worker.now(&doc).expect("the revision");
    worker.run(&read, "paragraph.untab", &["0", &at]).expect("the shown stop goes");
    assert_eq!(stops(&saved(&dir, &mut worker, &doc, "out.fodt")), 3, "one stop went");
}
