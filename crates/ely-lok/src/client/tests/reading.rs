use super::input::open;
use super::*;
use crate::review;

/// Three body paragraphs; the second carries a footnote.
pub(crate) const THREE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>one</text:p><text:p>two 二<text:note text:note-class="footnote"><text:note-citation>1</text:note-citation><text:note-body><text:p>aside</text:p></text:note-body></text:note></text:p><text:p>three</text:p></office:text></office:body>
</office:document>"#;

fn listed(worker: &mut Worker, doc: &Doc) -> Option<review::Body> {
    review::reading(&worker.values(doc, review::READING).expect("the list")).expect("a body")
}

#[test]
fn the_body_lists_whole_with_the_cursor_s_paragraph() {
    if install().is_none() {
        return eprintln!("SKIP: no pinned LOK build");
    }
    let (_dir, mut worker, doc) = open(THREE);
    let opened = worker.poll_events(&doc, 0, 250).expect("the first poll");
    assert_eq!(opened.a11y.text, "one", "the cursor starts in the first paragraph");
    // The footnote's own text stays out of the flow.
    let paragraphs = vec!["one".to_string(), "two 二1".into(), "three".into()];
    let at = |caret| Some(review::Body { paragraphs: paragraphs.clone(), caret });
    assert_eq!(listed(&mut worker, &doc), at(0));
    // Down: the second paragraph.
    worker.post_key(&doc, 0, 1024).expect("Down posts");
    let second = poll_until(&mut worker, &doc, &opened, HANG, |seen| seen.a11y.text.starts_with("two"));
    assert!(second.a11y.focused > opened.a11y.focused, "a new paragraph counts as a move");
    assert_eq!(listed(&mut worker, &doc), at(1));
    // A new footnote takes the cursor out of the body.
    worker.run(&doc, "note.footnote", &[]).expect("the footnote");
    poll_until(&mut worker, &doc, &second, HANG, |seen| seen.a11y.focused > second.a11y.focused);
    assert_eq!(listed(&mut worker, &doc), None, "a footnote names no body paragraph");
}
