//! DOC-01: files the macro check reads by their bytes.

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

#[test]
fn text_the_check_cannot_decode_still_opens() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let cases: [(&str, &[u8]); 3] = [
        ("utf32.txt", b"\xFF\xFE\0\0h\0\0\0i\0\0\0\n\0\0\0"),
        ("odd.txt", b"\xFF\xFEh\0i\0!"),
        ("lone.txt", b"\xFF\xFEh\0\x00\xD8i\0"),
    ];
    for (name, bytes) in cases {
        let path = dir.path().join(name);
        std::fs::write(&path, bytes).expect("the text writes");
        with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(path.clone(), cx));
        let opened = |app: &gpui::App| docs.read(app).core.tabs.iter().any(|tab| tab.name.as_ref() == name);
        settle(&cx, opened);
        cx.update(|app| {
            assert!(opened(app), "{name} opens");
            let tab = docs.read(app).active_tab().expect("a tab");
            assert!(tab.risk.is_none(), "{name} has no macros");
        });
    }
}

#[test]
fn utf32_flat_odf_with_basic_opens_in_risk_mode() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let body = super::more::macro_fodt(&mut cx).replace("encoding=\"UTF-8\"", "encoding=\"UTF-32\"");
    let before: Vec<u8> = [0xFF, 0xFE, 0, 0].into_iter().chain(body.chars().flat_map(|c| (c as u32).to_le_bytes())).collect();
    let path = dir.path().join("macro32.fodt");
    std::fs::write(&path, &before).expect("the file writes");
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(path.clone(), cx));
    settle(&cx, |app| docs.read(app).core.tabs.iter().any(|tab| tab.name.as_ref() == "macro32.fodt"));
    let id = cx.update(|app| {
        let tab = docs.read(app).active_tab().expect("a tab");
        assert!(tab.risk.is_some(), "Basic in UTF-32 is still Basic");
        tab.id
    });
    type_into(&mut cx, &docs, "x");
    with_window(&mut cx, &docs, |docs, window, cx| docs.save(id, None, window, cx));
    settle(&cx, |_| cx.did_prompt_for_new_path());
    cx.simulate_new_path_selection(|_| None);
    cx.run_until_parked();
    assert_eq!(std::fs::read(&path).expect("the original"), before, "the original stays");
}
