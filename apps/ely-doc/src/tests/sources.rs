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

/// Opens `fixture`, whose Word link stays a picture.
fn keeps_the_original(fixture: &str, password: &str) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office").join(fixture);
    let path = dir.path().join("main.docx");
    std::fs::copy(&source, &path).expect("copies");
    let original = std::fs::read(&path).expect("reads");
    let password = password.to_string();
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_with(path.clone(), password, cx));
    settle(&cx, |app| docs.read(app).active_tab().is_some());
    settled(&cx, &docs);
    let risk = cx.update(|app| docs.read(app).active_tab().expect("a tab").risk);
    assert_eq!(risk, Some(ely_office::Risk::Links), "{fixture}");
    let (ask, answered) = ely_ui::Ask::for_tests(&serde_json::json!(["save", path.display().to_string()]).to_string());
    cx.update(|app| docs.update(app, |docs, cx| docs.answer_request(ask, cx)));
    let refused = answered.try_recv().expect("answered at once");
    assert!(refused.as_ref().is_err_and(|why| why.contains("another name")), "{fixture}: {refused:?}");
    assert_eq!(std::fs::read(&path).expect("reads"), original, "{fixture}: the original stays");
}

#[test]
fn a_word_link_kept_as_a_picture_saves_only_as_a_copy() {
    keeps_the_original("ole-link-other.docx", "");
    keeps_the_original("ole-link-other-locked.docx", "ely");
}

#[test]
fn a_section_docx_keeps_only_as_text_is_said_and_never_written_over() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let (main, docx) = (dir.path().join("main.fodt"), dir.path().join("main.docx"));
    let href = ely_lok::file_url(&dir.path().join("include.docx"));
    let table = |inner: &str| format!("<table:table><table:table-column/><table:table-row><table:table-cell>{inner}</table:table-cell></table:table-row></table:table>");
    let section = format!(r#"<text:section text:name="S"><text:section-source xlink:href="{href}" xlink:type="simple"/>{}</text:section>"#, table("<text:p>Inner</text:p>"));
    std::fs::write(&main, format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:xlink="http://www.w3.org/1999/xlink"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text>{}</office:text></office:body>
</office:document>"#,
        table(&section)
    )).expect("writes the document");
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(main.clone(), cx));
    settle(&cx, |app| docs.read(app).active_tab().is_some());
    settled(&cx, &docs);
    let id = cx.update(|app| docs.read(app).active_tab().expect("a tab").id);
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, docx.clone(), None, window, cx));
    // The DOCX copy lost the link: the banner says so.
    settle(&cx, |app| docs.read(app).tab(id).is_some_and(|tab| tab.name == "main.docx" && tab.risk.is_some()));
    assert_eq!(cx.update(|app| docs.read(app).tab(id).expect("the tab").risk), Some(ely_office::Risk::Unkept));
    let written = std::fs::read(&docx).expect("reads");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.tab_mut(id).expect("the tab").risk = None;
        docs.save(id, None, window, cx);
    });
    settle(&cx, |app| docs.read(app).tab(id).is_some_and(|tab| tab.risk.is_some()));
    assert_eq!(cx.update(|app| docs.read(app).tab(id).expect("the tab").risk), Some(ely_office::Risk::Unkept));
    assert_eq!(std::fs::read(&docx).expect("reads"), written, "the DOCX is never saved over");
}

/// A one-cell table holding `inner`.
fn cell(inner: &str) -> String {
    format!("<table:table><table:table-column/><table:table-row><table:table-cell>{inner}</table:table-cell></table:table-row></table:table>")
}

/// A text document whose body is `body`, tables allowed.
fn tabled(path: &Path, body: &str) {
    std::fs::write(path, format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:xlink="http://www.w3.org/1999/xlink"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text>{body}</office:text></office:body>
</office:document>"#
    )).expect("writes the document");
}

#[test]
fn a_program_saving_a_section_unkept_since_an_update_hears_at_once() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let (main, docx, include) = (dir.path().join("main.fodt"), dir.path().join("main.docx"), dir.path().join("include.fodt"));
    // A source opening with a table: updated, it is unkept.
    tabled(&include, &format!("{}<text:p>Tail</text:p>", cell("<text:p>Grid</text:p>")));
    let href = ely_lok::file_url(&include);
    let section = format!(r#"<text:section text:name="S"><text:section-source xlink:href="{href}" xlink:type="simple"/><text:p>Cached</text:p></text:section>"#);
    tabled(&main, &cell(&section));
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(main.clone(), cx));
    settle(&cx, |app| docs.read(app).active_tab().is_some());
    settled(&cx, &docs);
    let id = cx.update(|app| docs.read(app).active_tab().expect("a tab").id);
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, docx.clone(), None, window, cx));
    settle(&cx, |app| docs.read(app).tab(id).is_some_and(|tab| tab.name == "main.docx" && !tab.exporting));
    assert_eq!(cx.update(|app| docs.read(app).tab(id).expect("the tab").risk), None, "one paragraph fits the cell");
    cx.update(|app| docs.update(app, |docs, cx| docs.update_links(id, cx)));
    linked(&cx, &docs, Linked::Updated(Vec::new()));
    let written = std::fs::read(&docx).expect("reads");
    let (ask, answered) = ely_ui::Ask::for_tests(&serde_json::json!(["save", docx.display().to_string()]).to_string());
    cx.update(|app| docs.update(app, |docs, cx| docs.answer_request(ask, cx)));
    let answer = std::cell::RefCell::new(None);
    settle(&cx, |_| {
        let mut got = answer.borrow_mut();
        if got.is_none() {
            *got = answered.try_recv().ok();
        }
        got.is_some()
    });
    let refused = answer.into_inner().expect("answered");
    assert!(refused.as_ref().is_err_and(|why| why.contains("another name")), "{refused:?}");
    assert_eq!(cx.update(|app| docs.read(app).tab(id).expect("the tab").risk), Some(ely_office::Risk::Unkept));
    assert!(!cx.did_prompt_for_new_path(), "a program's save opened a panel");
    assert_eq!(std::fs::read(&docx).expect("reads"), written, "the DOCX is never saved over");
}
