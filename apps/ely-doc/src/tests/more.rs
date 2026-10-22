use gpui::TestAppContext;

use super::*;

/// The report template with an embedded Basic library.
fn macro_fodt(cx: &mut TestAppContext) -> String {
    cx.update(|app| Template::Report.fodt(app))
        .replace(
            "office:version=",
            "xmlns:script=\"urn:oasis:names:tc:opendocument:xmlns:script:1.0\" xmlns:ooo=\"http://openoffice.org/2004/office\" office:version=",
        )
        .replace(
            "<office:body>",
            "<office:scripts><office:script script:language=\"ooo:Basic\"><ooo:libraries><ooo:library-embedded ooo:name=\"Standard\"><ooo:module ooo:name=\"Module1\"><ooo:source-code>Sub Main&#x0A;End Sub</ooo:source-code></ooo:module></ooo:library-embedded></ooo:libraries></office:script></office:scripts><office:body>",
        )
}

#[test]
fn zip_macro_checks_read_real_kernel_files() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let source = dir.path().join("basic.fodt");
    std::fs::write(&source, macro_fodt(&mut cx)).expect("writes");
    let worker = cx.update(|app| docs.read(app).core.worker.clone());
    let mut worker = worker.lock().expect("the worker lock");
    let doc = worker.load(&ely_lok::file_url(&source)).expect("loads");
    let (odt, docx) = (dir.path().join("basic.odt"), dir.path().join("plain.docx"));
    doc.save_as(&mut worker, &ely_lok::file_url(&odt), "odt")
        .expect("odt");
    doc.save_as(&mut worker, &ely_lok::file_url(&docx), "docx")
        .expect("docx");
    worker.close_doc(doc).expect("closes");
    assert!(
        ely_office::files::macro_risk(&Docs::KIND, &odt).expect("odt"),
        "an ODT with Basic is a risk"
    );
    assert!(
        !ely_office::files::macro_risk(&Docs::KIND, &docx).expect("docx"),
        "a DOCX drops the Basic"
    );
}

#[test]
fn a_local_template_with_macros_opens_in_risk_mode() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let template = dir.path().join("macro-template.fodt");
    std::fs::write(&template, macro_fodt(&mut cx)).expect("writes");
    with_window(&mut cx, &docs, |docs, _, cx| {
        docs.new_from_file(&template, cx)
    });
    settle(&cx, |app| docs.read(app).core.active.is_some());
    cx.update(|app| {
        assert!(
            docs.read(app).active_tab().expect("a tab").risk.is_some(),
            "the copy shows the risk"
        )
    });
}

#[test]
fn select_all_alone_leaves_the_tab_clean() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &docs);
    let view = cx.update(|app| docs.read(app).active_tab().expect("a tab").view.clone());
    view.update(&mut cx, |view, _| {
        view.run("edit.select_all", &[]).expect("select all queues")
    });
    settle(&cx, |app| view.read(app).settled());
    for _ in 0..40 {
        cx.executor().tick();
    }
    cx.update(|app| {
        assert!(
            !docs.read(app).active_tab().expect("a tab").dirty(app),
            "selection is no edit"
        )
    });
}

#[test]
fn keep_mine_waits_and_two_queued_saves_never_crash() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    let mine = dir.path().join("mine.docx");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, mine.clone(), None, window, cx)
    });
    written(&cx, &mine, None);
    type_into(&mut cx, &docs, "x");
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&mine, "EXTERNAL").expect("the outside write");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.save(id, None, window, cx)
    });
    commit(&mut cx, &docs, "PendingKeepMine");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.keep_mine(id, Some(After::Tab), window, cx)
    });
    settle(&cx, |app| docs.read(app).core.tabs.is_empty());
    let saved = text_of(&mut cx, &docs, &mine);
    assert!(
        saved.contains("PendingKeepMine"),
        "keep mine saved the queued text: {}",
        saved[saved.find("<office:text").unwrap_or(0)..].replace("\n", " ")
    );

    let id = blank(&mut cx, &docs);
    commit(&mut cx, &docs, "queued");
    let (one, two) = (dir.path().join("one.docx"), dir.path().join("two.docx"));
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, one.clone(), Some(After::Tab), window, cx)
    });
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, two.clone(), None, window, cx)
    });
    settle(&cx, |app| docs.read(app).core.tabs.is_empty());
    for _ in 0..40 {
        cx.executor().tick();
    }
    assert!(
        one.exists() && !two.exists(),
        "the closed tab saves no more"
    );
}

#[test]
fn a_kernel_write_failure_stays_visible() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "RetainedText");
    let work = cx.update(|app| docs.read(app).active_tab().expect("a tab").work.clone());
    std::fs::remove_dir_all(&work).expect("drops the scratch");
    std::fs::write(&work, b"not a folder").expect("blocks it");
    let target = dir.path().join("failure.docx");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, target.clone(), None, window, cx)
    });
    settle(&cx, |app| {
        let tab = docs.read(app).active_tab().expect("a tab");
        !tab.exporting && tab.session.state() == ely_files::SaveState::SaveFailed
    });
    assert!(!target.exists(), "the failure writes nothing");
    std::fs::remove_file(&work).expect("unblocks");
    std::fs::create_dir_all(&work).expect("the scratch returns");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, target.clone(), None, window, cx)
    });
    written(&cx, &target, None);
    assert!(target.exists(), "a retry lands");
    cx.update(|app| {
        let state = docs.read(app).active_tab().expect("a tab").session.state();
        assert_eq!(
            state,
            ely_files::SaveState::Saved,
            "the retry clears the failure"
        );
    });
}

#[test]
fn an_edit_that_just_landed_still_asks_and_selection_does_not() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    commit(&mut cx, &docs, "JustLanded");
    let view = cx.update(|app| docs.read(app).active_tab().expect("a tab").view.clone());
    settle(&cx, |app| view.read(app).settled());
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.request_close(id, After::Tab, window, cx)
    });
    settle(&cx, |app| {
        matches!(docs.read(app).core.dialog, Some(Dialog::Unsaved { .. }))
    });
    assert!(
        cx.update(|app| docs.read(app).tab(id).is_some()),
        "a landed edit asks before closing"
    );

    let id = blank(&mut cx, &docs);
    let view = cx.update(|app| docs.read(app).active_tab().expect("a tab").view.clone());
    view.update(&mut cx, |view, _| {
        view.run("edit.select_all", &[]).expect("select all queues")
    });
    settle(&cx, |app| view.read(app).settled());
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.request_close(id, After::Tab, window, cx)
    });
    settle(&cx, |app| docs.read(app).tab(id).is_none());
}

#[test]
fn a_quit_from_either_window_asks_first() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    commit(&mut cx, &docs, "Unsaved");
    let view = cx.update(|app| docs.read(app).active_tab().expect("a tab").view.clone());
    settle(&cx, |app| view.read(app).settled());
    cx.update(ely_ui::commands::bind_quit);
    let home = cx.add_window(|_, _| Blank);
    cx.update_window(home.into(), |_, window, app| docs.update(app, |docs, cx| docs.own_window(window, cx)))
        .expect("the window lives");
    // The settings window has no quit of its own.
    let settings = cx.add_window(|_, _| Blank);
    for from in [home.into(), settings.into()] {
        cx.dispatch_action(from, ely_ui::commands::Quit);
        settle(&cx, |app| matches!(docs.read(app).core.dialog, Some(Dialog::Unsaved { .. })));
        assert!(cx.update(|app| docs.read(app).tab(id).is_some()), "the tab waits for an answer");
        assert_eq!(cx.update(|app| app.active_window()), Some(home.into()), "the question's window comes forward");
        // Cancel, as the dialog's button does.
        cx.update(|app| docs.update(app, |docs, cx| {
            docs.close_cancelled(cx);
            docs.core.dialog = None;
        }));
    }
}

#[test]
fn closing_a_tab_takes_its_export_dialog() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    cx.update(|app| docs.update(app, |docs, cx| docs.ask_output(false, cx)));
    assert!(cx.update(|app| matches!(docs.read(app).dialog, Some(crate::actions::Dialog::Output { .. }))));
    with_window(&mut cx, &docs, |docs, window, cx| docs.request_close(id, After::Tab, window, cx));
    settle(&cx, |app| docs.read(app).tab(id).is_none());
    // Drawn now, a dialog for the gone tab would crash.
    assert!(cx.update(|app| docs.read(app).dialog.is_none()), "the dialog went with its tab");
}

#[test]
fn a_tab_on_a_vanished_folder_takes_no_other_file() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    let gone = dir.path().join("volume");
    std::fs::create_dir(&gone).expect("a folder");
    let first = gone.join("first.docx");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, first.clone(), None, window, cx));
    written(&cx, &first, None);
    // Its volume goes away; another file still saves.
    std::fs::remove_dir_all(&gone).expect("the folder goes");
    let other = dir.path().join("other.docx");
    let holder = cx.update(|app| docs.read(app).holder(&other).map_err(|error| error.to_string()));
    assert_eq!(holder, Ok(None));
}

#[test]
fn an_edit_after_a_save_raced_it_counts_again() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "a");
    settle(&cx, |app| docs.read(app).tab(id).is_some_and(|tab| tab.modified));
    // A save's ticket landed after this edit: saved, still modified.
    cx.update(|app| docs.update(app, |docs, _| docs.tab_mut(id).expect("a tab").session.reverted()));
    type_into(&mut cx, &docs, "b");
    settle(&cx, |app| docs.read(app).tab(id).is_some_and(|tab| tab.session.state() == ely_files::SaveState::Dirty));
}

#[test]
fn the_side_panel_hides_and_review_brings_it_back() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &docs);
    let shown = |cx: &mut TestAppContext| cx.update(|app| docs.update(app, |docs, cx| docs.side_view(cx).is_some()));
    assert!(shown(&mut cx), "open at first");
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_side_panel(&crate::docs::ToggleSidePanel, window, cx));
    assert!(!shown(&mut cx), "hidden");
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_show_review(&crate::review::ShowReview, window, cx));
    assert!(shown(&mut cx), "review opens it");
    assert_eq!(cx.update(|app| docs.read(app).side), crate::docs::Side::Review);
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_side_panel(&crate::docs::ToggleSidePanel, window, cx));
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_show_review(&crate::review::ShowReview, window, cx));
    assert_eq!(cx.update(|app| docs.read(app).side), crate::docs::Side::Review, "a hidden review comes back as review");
    assert!(shown(&mut cx));
}

#[test]
fn a_default_zoom_opens_each_document_at_it() {
    use ely_platform::settings::DocZoom;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let zoom = |cx: &mut TestAppContext| cx.update(|app| docs.read(app).active_tab().expect("a tab").view.read(app).zoom());
    blank(&mut cx, &docs);
    assert_eq!(zoom(&mut cx), 1.0, "automatic is actual size");
    cx.update(|app| app.global_mut::<ely_ui::settings::Current>().shared.doc_zoom = DocZoom::Percent(150));
    blank(&mut cx, &docs);
    assert_eq!(zoom(&mut cx), 1.5);
}
