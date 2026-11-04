//! Merging empty cells merges them (#12).

use super::*;

#[test]
fn empty_cells_merge() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &book);
    update(&mut cx, &book, |book, cx| book.run_active("cell.goto", &["E1:F1"], cx));
    settle(&cx, |app| {
        book.read(app).active_tab().is_some_and(|tab| {
            tab.view.read(app).settled() && tab.view.read(app).cell().address == "E1:F1"
        })
    });
    update(&mut cx, &book, |book, cx| book.merge(book.core.active(), cx));
    let path = dir.path().join("merged.fods");
    // Saves until the merge shows in the file.
    settle(&cx, |app| {
        let book = book.read(app);
        let mut worker = book.core.worker.lock().expect("the worker lock");
        book.core.tabs[0]
            .doc
            .save_as(&mut worker, &ely_lok::file_url(&path), "fods")
            .expect("the save");
        let xml = std::fs::read_to_string(&path).expect("the saved file");
        xml.contains("table:number-columns-spanned=\"2\"")
    });
    assert!(cx.update(|app| book.read(app).dialog.is_none()), "nothing asks: no cell holds data");
}
