use ely_office::After;
use gpui::TestAppContext;

use super::*;
use crate::merge::read_csv;

#[test]
fn typing_during_a_closing_save_is_never_lost() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "BeforeExport ");
    let path = dir.path().join("closing.odt");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, path.clone(), Some(After::Tab), window, cx)
    });
    // The file is on disk; its landing has not run.
    let exporting = |app: &gpui::App| docs.read(app).tab(id).is_some_and(|tab| tab.exporting);
    while !path.exists() {
        assert!(cx.update(|app| exporting(app)), "the save ended before its file was seen");
        assert!(cx.executor().tick(), "the save stalled");
    }
    assert!(cx.update(|app| exporting(app)), "the landing waits");
    commit(&mut cx, &docs, "AfterExport");
    asks_again(&mut cx, &docs, id, &path);
}

#[test]
fn typing_that_lands_during_a_closing_save_is_never_lost() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let dispatcher = gpui::TestDispatcher::new(0);
    let mut cx = TestAppContext::build(dispatcher.clone(), None);
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "BeforeExport ");
    let (copy, view) = cx.update(|app| {
        let tab = docs.read(app).tab(id).expect("the tab");
        (tab.work.join("save.odt"), tab.view.clone())
    });
    assert!(!copy.exists(), "no export yet");
    let start = cx.update(|app| view.read(app).edited());
    let path = dir.path().join("closing.odt");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, path.clone(), Some(After::Tab), window, cx)
    });
    // The kernel's copy is read; nothing has landed.
    while !copy.exists() {
        assert!(cx.executor().tick(), "the export stalled");
    }
    commit(&mut cx, &docs, "AfterExport");
    // The key reaches the kernel; windows wait.
    while dispatcher.tick(true) {}
    let exporting = |app: &gpui::App| docs.read(app).tab(id).is_some_and(|tab| tab.exporting);
    while cx.update(|app| exporting(app)) {
        assert!(cx.executor().tick(), "the save stalled");
    }
    let (pending, edited) = cx.update(|app| (view.read(app).edit_pending(), view.read(app).edited()));
    assert!(!pending && edited > start, "the key landed before the save: {pending}, {start} to {edited}");
    asks_again(&mut cx, &docs, id, &path);
}

/// The closing save kept the later typing and asked again.
fn asks_again(cx: &mut TestAppContext, docs: &Entity<Docs>, id: u64, path: &std::path::Path) {
    settle(cx, |app| {
        let docs = docs.read(app);
        docs.tab(id).is_none() || matches!(docs.core.dialog, Some(Dialog::Unsaved { .. }))
    });
    let open = cx.update(|app| docs.read(app).tab(id).is_some());
    assert!(open, "a close after later typing asks again");
    let text = text_of(cx, docs, path);
    assert!(text.contains("BeforeExport"), "the export saved");
    assert!(!text.contains("AfterExport"), "later typing came after the export");
}

#[test]
fn two_tabs_saving_as_one_path_keep_one_owner() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let first = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "FirstOwner");
    let second = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "SecondOwner");
    let same = dir.path().join("same.odt");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(first, same.clone(), None, window, cx);
        docs.write_as(second, same.clone(), None, window, cx);
    });
    written(&cx, &same, None);
    settle(&cx, |app| {
        docs.read(app).core.tabs.iter().all(|tab| !tab.exporting)
    });
    let owners: Vec<u64> = cx.update(|app| {
        let docs = docs.read(app);
        docs.core
            .tabs
            .iter()
            .filter(|tab| {
                tab.session
                    .path()
                    .is_some_and(|path| ely_files::same_file(path, &same).expect("comparable"))
            })
            .map(|tab| tab.id)
            .collect()
    });
    assert_eq!(owners.len(), 1, "one tab holds the file");
    let text = text_of(&mut cx, &docs, &same);
    let mark = if owners[0] == first {
        "FirstOwner"
    } else {
        "SecondOwner"
    };
    assert!(text.contains(mark), "the file is its owner's");
}

#[test]
fn interleaved_previews_keep_their_sources() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let alpha = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "SourceAlpha ");
    update_doc(&mut cx, &docs, |docs, cx| docs.field_here("Name", cx));
    let beta = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "SourceBeta ");
    update_doc(&mut cx, &docs, |docs, cx| docs.field_here("Name", cx));
    settled(&cx, &docs);
    let csv = dir.path().join("two.csv");
    std::fs::write(&csv, "Name\nAda\nGrace\n").expect("the csv writes");
    let table = read_csv(&csv).expect("the csv reads");
    update_doc(&mut cx, &docs, |docs, cx| {
        docs.preview_record(alpha, &table, 0, cx);
        docs.preview_record(beta, &table, 1, cx);
    });
    settle(&cx, |app| {
        let docs = docs.read(app);
        // A filled preview is a modified copy.
        docs.core.tabs.len() == 4 && docs.core.tabs[2..].iter().all(|tab| tab.modified)
    });
    let previews: Vec<u64> = cx.update(|app| {
        let tabs = &docs.read(app).core.tabs;
        tabs[2..].iter().map(|tab| tab.id).collect()
    });
    let mut texts = Vec::new();
    for (n, id) in previews.into_iter().enumerate() {
        let path = dir.path().join(format!("preview{n}.odt"));
        with_window(&mut cx, &docs, |docs, window, cx| {
            docs.write_as(id, path.clone(), None, window, cx)
        });
        written(&cx, &path, None);
        texts.push(text_of(&mut cx, &docs, &path));
    }
    assert!(
        texts.iter().any(|text| text.contains("SourceAlpha Ada")),
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|text| text.contains("SourceBeta Grace")),
        "{texts:?}"
    );
}

#[test]
fn closing_a_clean_tab_never_waits_for_the_worker() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    settled(&cx, &docs);
    let worker = cx.update(|app| docs.read(app).core.worker.clone());
    let (held, hold) = std::sync::mpsc::channel();
    let holder = std::thread::spawn(move || {
        let _busy = worker.lock().expect("the worker lock");
        held.send(()).expect("the test waits");
        // The engine stays held this long.
        std::thread::sleep(std::time::Duration::from_millis(400));
    });
    hold.recv().expect("the lock is held");
    let started = std::time::Instant::now();
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.request_close(id, After::Tab, window, cx)
    });
    let took = started.elapsed();
    assert!(
        took < std::time::Duration::from_millis(100),
        "the close waited {took:?}"
    );
    holder.join().expect("the holder ends");
    settle(&cx, |app| docs.read(app).tab(id).is_none());
}

fn update_doc(
    cx: &mut TestAppContext,
    docs: &Entity<Docs>,
    f: impl FnOnce(&mut Docs, &mut gpui::Context<Docs>),
) {
    cx.update(|app| docs.update(app, f));
}
