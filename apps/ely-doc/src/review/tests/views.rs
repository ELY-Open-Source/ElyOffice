//! #126: Markup, Original and Final change only the display.

use super::*;

/// The body's paragraph texts, deleted text left out.
fn body(cx: &mut TestAppContext, docs: &Entity<Docs>, doc: &ely_lok::Doc) -> Vec<String> {
    let worker = cx.update(|app| docs.read(app).core.worker.clone());
    let json = worker.lock().expect("the worker lock").values(doc, ely_lok::review::BODY).expect("the body reads");
    ely_lok::review::body(&json).expect("the body parses").into_iter().map(|p| p.text).collect()
}

#[test]
fn the_three_views_show_the_document_and_keep_every_change() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Start");
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_track(&TrackChanges, window, cx));
    type_into(&mut cx, &docs, " Alpha");
    let review = panel(&mut cx, &docs);
    settle(&cx, |app| review.read(app).changes.len() == 1);
    let (view, doc) = cx.update(|app| {
        let tab = docs.read(app).active_tab().expect("a tab");
        (tab.view.clone(), tab.doc.clone())
    });
    let shows = |app: &gpui::App| view.read(app).state(".uno:ShowTrackedChanges").map(str::to_owned);
    let show_as = |cx: &mut TestAppContext, how: &'static str| {
        cx.update(|app| review.update(app, |review, cx| review.show_as(how, cx)));
    };
    show_as(&mut cx, "final");
    settle(&cx, |app| shows(app).as_deref() == Some("false"));
    // Original: a read-only copy with every change rejected.
    show_as(&mut cx, "original");
    settle(&cx, |app| review.read(app).original.is_some());
    let copy = cx.update(|app| review.read(app).original.as_ref().expect("the copy").doc.clone());
    assert_eq!(body(&mut cx, &docs, &copy), ["Start"], "the copy has no insertion");
    cx.update(|app| {
        let worker = docs.read(app).core.worker.clone();
        worker.lock().expect("the worker lock").post_key(&copy, 'z' as u32, 512).expect("z posts");
    });
    assert_eq!(body(&mut cx, &docs, &copy), ["Start"], "the copy takes no typing");
    // Back to Markup: the copy goes, the tab shows changes.
    show_as(&mut cx, "markup");
    settle(&cx, |app| review.read(app).original.is_none() && shows(app).as_deref() == Some("true"));
    assert_eq!(body(&mut cx, &docs, &doc), ["Start Alpha"], "the document keeps its words");
    assert_eq!(cx.update(|app| review.read(app).changes.len()), 1, "the change stays tracked");
    // A saved copy still carries the change.
    let path = dir.path().join("kept.fodt");
    cx.update(|app| {
        let worker = docs.read(app).core.worker.clone();
        doc.save_as(&mut worker.lock().expect("the worker lock"), &ely_lok::file_url(&path), "fodt").expect("saves");
    });
    let xml = std::fs::read_to_string(&path).expect("the save reads");
    assert!(xml.contains("<text:changed-region") && xml.contains("Alpha"), "the change is saved");
}

#[test]
fn a_document_without_changes_shows_its_original_too() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Plain");
    let review = panel(&mut cx, &docs);
    cx.update(|app| review.update(app, |review, cx| review.show_as("original", cx)));
    settle(&cx, |app| review.read(app).original.is_some() || !review.read(app).preparing);
    let copy = cx.update(|app| review.read(app).original.as_ref().map(|original| original.doc.clone()));
    let copy = copy.expect("the copy opens with no changes to reject");
    assert_eq!(body(&mut cx, &docs, &copy), ["Plain"]);
}

#[test]
fn changes_only_pairs_a_replacement_with_its_paragraph() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Price 5");
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_track(&TrackChanges, window, cx));
    settle(&cx, |app| {
        let tab = docs.read(app).active_tab().expect("a tab");
        tab.view.read(app).state(".uno:TrackChanges") == Some("true")
    });
    let review = panel(&mut cx, &docs);
    // Backspace deletes the 5; typing inserts beside it.
    cx.update(|app| {
        let docs = docs.read(app);
        let doc = docs.active_tab().expect("a tab").doc.clone();
        docs.core.worker.lock().expect("the worker lock").post_key(&doc, 0, 1283).expect("backspace posts");
    });
    settle(&cx, |app| review.read(app).changes.len() == 1);
    type_into(&mut cx, &docs, "10");
    settle(&cx, |app| review.read(app).changes.len() == 2);
    cx.update(|app| review.update(app, |review, cx| review.show_as("changes", cx)));
    let (rows, paragraph) = cx.update(|app| {
        let review = review.read(app);
        let rows: Vec<Option<(String, String)>> = only::rows(&review.changes)
            .iter()
            .map(|row| row.replaced().map(|(old, new)| (old.to_string(), new.to_string())))
            .collect();
        (rows, review.changes[0].paragraph.clone())
    });
    assert_eq!(rows, [Some(("5".to_string(), "10".to_string()))], "one row, old to new");
    assert!(paragraph.is_some_and(|text| only::context(&text).starts_with("Price")), "the paragraph's start");
    assert!(cx.update(|app| review.read(app).only), "the list shows changes only");
    let shows = cx.update(|app| {
        let tab = docs.read(app).active_tab().expect("a tab");
        tab.view.read(app).state(".uno:ShowTrackedChanges").map(str::to_owned)
    });
    assert_eq!(shows.as_deref(), Some("true"), "the page keeps its markup");
}
