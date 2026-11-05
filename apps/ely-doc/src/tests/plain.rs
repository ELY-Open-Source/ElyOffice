//! DOC-01: text that only looks like markup.

use gpui::TestAppContext;

use super::*;

#[test]
fn text_led_by_a_bracket_opens_and_saves_as_text() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let notes = dir.path().join("notes.txt");
    let before = b"<3 my notes\nsecond line\n";
    std::fs::write(&notes, before).expect("the notes write");
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(notes.clone(), cx));
    settle(&cx, |app| docs.read(app).core.active().is_some());
    let id = cx.update(|app| {
        let tab = docs.read(app).active_tab().expect("a tab opens");
        assert!(tab.risk.is_none(), "plain text has no macros");
        tab.id
    });
    type_into(&mut cx, &docs, "x");
    with_window(&mut cx, &docs, |docs, window, cx| docs.save(id, None, window, cx));
    written(&cx, &notes, Some(before));
    let after = std::fs::read_to_string(&notes).expect("still text");
    assert!(after.contains("<3 my notes") && after.contains("second line"), "{after:?}");
}
