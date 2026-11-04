//! LOK-11: a linked section reads its file only when asked.

use ely_office::Linked;

use super::*;

/// A text document; `section` links `include` and shows `cached`.
fn fodt(path: &Path, said: &str, section: Option<(&Path, &str)>) {
    let linked = section.map_or_else(String::new, |(include, cached)| {
        let href = ely_lok::file_url(include);
        format!(
            r#"<text:section text:name="S"><text:section-source xlink:href="{href}" xlink:type="simple"/><text:p>{cached}</text:p></text:section>"#
        )
    });
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:xlink="http://www.w3.org/1999/xlink"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>{said}</text:p>{linked}</office:text></office:body>
</office:document>"#
    );
    std::fs::write(path, xml).expect("writes");
}

/// The open document as the kernel holds it now.
fn shown(cx: &mut TestAppContext, docs: &Entity<Docs>, dir: &Path) -> String {
    let flat = dir.join("shown.fodt");
    cx.update(|app| {
        let docs = docs.read(app);
        let tab = docs.active_tab().expect("a tab");
        let mut worker = docs.core.worker.lock().expect("the worker lock");
        tab.doc.save_as(&mut worker, &ely_lok::file_url(&flat), "fodt").expect("a copy");
    });
    std::fs::read_to_string(&flat).expect("reads")
}

#[track_caller]
fn linked(cx: &TestAppContext, docs: &Entity<Docs>, want: Linked) {
    settle(cx, |app| docs.read(app).active_tab().is_some_and(|tab| *tab.links.state() == want));
}

#[test]
fn a_linked_section_updates_when_asked_and_says_when_its_file_changes() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let (main, include) = (dir.path().join("main.fodt"), dir.path().join("include.fodt"));
    fodt(&include, "One", None);
    fodt(&main, "Before", Some((&include, "Cached")));
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(main.clone(), cx));
    linked(&cx, &docs, Linked::Saved(Vec::new()));
    let (id, sources) = cx.update(|app| {
        let tab = docs.read(app).active_tab().expect("the tab");
        (tab.id, tab.links.sources().to_vec())
    });
    assert_eq!(sources, [ely_lok::file_url(&include)]);
    assert!(shown(&mut cx, &docs, dir.path()).contains(">Cached<"), "opening shows the saved text");
    cx.update(|app| docs.update(app, |docs, cx| docs.update_links(id, cx)));
    linked(&cx, &docs, Linked::Updated(Vec::new()));
    assert!(shown(&mut cx, &docs, dir.path()).contains(">One<"), "the update reads the file");
    fodt(&include, "Two", None);
    linked(&cx, &docs, Linked::Changed(include.display().to_string()));
    assert!(shown(&mut cx, &docs, dir.path()).contains(">One<"), "a later save waits for the user");
}
