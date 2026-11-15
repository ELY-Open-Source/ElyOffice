//! Kept results of unknown functions, counted in the status (D-02).

use std::path::Path;

use super::data::run;
use super::*;

fn kept(cx: &TestAppContext, book: &Entity<Book>, want: &[&str]) {
    settle(cx, |app| book.read(app).active_tab().is_some_and(|tab| tab.extra.kept == want));
}

#[test]
fn the_kept_list_follows_the_cells_showing_the_files_results() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    // The window's toasts live on its root.
    let root = cx
        .update_window(window.into(), |_, window, app| {
            let root = ely_ui::document_root(ely_platform::AppId::Table, book.clone().into(), &ely_ui::commands::FILE, Vec::new(), window, app);
            book.update(app, |book, _| book.core.root = root.downgrade());
            root
        })
        .expect("the window lives");
    // A copy: the kernel locks what it opens.
    let path = dir.path().join("kept.xlsx");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/table/kept.xlsx");
    std::fs::copy(fixture, &path).expect("copies");
    update(&mut cx, &book, |book, cx| book.open_file(path.clone(), cx));
    kept(&cx, &book, &["Sheet1!B1", "Sheet1!B2", "Sheet1!B3", "Sheet1!D1", "Sheet1!E1"]);
    // The menu lists them and says the file loses nothing.
    update(&mut cx, &book, |book, cx| book.list_kept(cx));
    cx.update_window(window.into(), |_, window, app| book.update(app, |book, cx| book.on_show_risk(&ely_office::ShowRisk, window, cx)))
        .expect("the window lives");
    let said: Vec<_> = cx.update(|app| root.read(app).toasts(app)).into_iter().map(|(text, _)| text).collect();
    assert!(said.iter().any(|text| text.contains("Sheet1!D1")), "the kept cells: {said:?}");
    assert!(said.iter().any(|text| text.contains("loses nothing")), "no risk: {said:?}");
    run(&mut cx, &book, "cell.goto", &["A1"]);
    run(&mut cx, &book, "cell.enter", &["3"]);
    kept(&cx, &book, &["Sheet1!B1", "Sheet1!B2", "Sheet1!B3", "Sheet1!E1"]);
}

#[test]
fn a_long_list_names_five_and_counts_all() {
    let cells = |n: usize| (1..=n).map(|row| format!("S!A{row}")).collect::<Vec<_>>();
    assert_eq!(crate::book::listed(&cells(2)), "S!A1, S!A2");
    assert_eq!(crate::book::listed(&cells(7)), "S!A1, S!A2, S!A3, S!A4, S!A5, … (7)");
}
