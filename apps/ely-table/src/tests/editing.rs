//! Saves mid-edit leave the typing in the cell (R15.39).

use ely_lok::sheet;
use gpui::VisualTestContext;

use super::undo::drawn;
use super::*;

/// Waits until the formula bar shows `text`.
fn shown(cx: &VisualTestContext, book: &Entity<Book>, text: &str) {
    settle(cx, |app| {
        book.read(app).active_tab().is_some_and(|tab| {
            let view = tab.view.read(app);
            view.settled() && view.cell().formula == text
        })
    });
}

/// `cells` of the file at `path`, read from a copy.
fn on_disk(cx: &mut VisualTestContext, book: &Entity<Book>, path: &Path, cells: &[&str]) -> Vec<String> {
    let copy = path.with_file_name("read.xlsx");
    std::fs::copy(path, &copy).expect("the saved file copies");
    cx.update(|_, app| {
        let mut worker = book.read(app).core.worker.lock().expect("the worker lock");
        let doc = worker.load(&ely_lok::file_url(&copy)).expect("the saved file loads");
        let read = cells
            .iter()
            .map(|cell| sheet::cell_at(&worker.values(&doc, &sheet::cell_at_query(cell)).expect("reads")).expect("a cell").text)
            .collect();
        worker.close_doc(doc).expect("closes");
        read
    })
}

/// A blank workbook saved at `path`, the grid focused.
fn saved_book(cx: &mut VisualTestContext, book: &Entity<Book>, path: &Path) -> u64 {
    blank(cx, book);
    let id = cx.update(|_, app| book.read(app).core.tabs[0].id);
    cx.update(|window, app| book.update(app, |book, cx| book.write_as(id, path.to_path_buf(), None, window, cx)));
    settle(cx, |app| book.read(app).core.tabs[0].name == "book.xlsx" && !book.read(app).exporting(id));
    cx.update(|window, app| {
        let focus = book.read(app).core.tabs[0].view.read(app).focus_handle().clone();
        window.focus(&focus, app);
    });
    id
}

#[test]
fn a_save_mid_edit_keeps_the_typing_and_a_closing_save_enters_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((book, cx)) = drawn(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("book.xlsx");
    let id = saved_book(cx, &book, &path);
    let sheet = cx.update(|_, app| sheet_names(app, &book)[0].clone());
    cx.simulate_keystrokes("a b c");
    shown(cx, &book, "abc");
    // Autosave and other programs' saves take this path.
    cx.update(|window, app| book.update(app, |book, cx| book.save(id, None, window, cx)));
    settle(cx, |app| !book.read(app).exporting(id));
    assert!(cx.update(|_, app| book.read(app).core.tabs[0].dirty(app)), "the open edit stays unsaved");
    cx.simulate_keystrokes("d e f");
    shown(cx, &book, "abcdef");
    let (a1, a2) = (format!("{sheet}!A1"), format!("{sheet}!A2"));
    assert_eq!(on_disk(cx, &book, &path, &[&a1]), [""], "the save wrote A1 as entered");
    cx.simulate_keystrokes("enter g h");
    shown(cx, &book, "gh");
    cx.update(|window, app| book.update(app, |book, cx| book.save(id, Some(ely_office::After::Tab), window, cx)));
    settle(cx, |app| book.read(app).core.tabs.is_empty());
    assert_eq!(on_disk(cx, &book, &path, &[&a1, &a2]), ["abcdef", "gh"], "the closing save entered A2");
}

#[test]
fn a_closing_save_keeps_the_tab_when_the_entry_is_refused() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((book, cx)) = drawn(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("book.xlsx");
    let id = saved_book(cx, &book, &path);
    let rule = sheet::Validation {
        kind: "list".into(),
        operator: "equal".into(),
        list: vec!["Red".into()],
        show_error: true,
        error_style: "stop".into(),
        ..sheet::Validation::default()
    };
    update(cx, &book, |book, cx| book.run_active("rules.validation", &[&rule.to_json()], cx));
    cx.simulate_keystrokes("a b c");
    shown(cx, &book, "abc");
    let before = std::fs::read(&path).expect("the saved file");
    cx.update(|window, app| book.update(app, |book, cx| book.save(id, Some(ely_office::After::Tab), window, cx)));
    settle(cx, |app| !book.read(app).exporting(id));
    let state = cx.update(|_, app| book.read(app).core.tabs.first().map(|tab| tab.session.state()));
    assert_eq!(state, Some(ely_files::SaveState::SaveFailed), "the tab stays and says the save failed");
    shown(cx, &book, "abc");
    assert_eq!(std::fs::read(&path).expect("the file"), before, "nothing is written");
}
