//! Kept results of unknown functions, named by a banner (D-02).

use std::path::Path;

use super::data::run;
use super::*;

fn kept(cx: &TestAppContext, book: &Entity<Book>, want: &[&str]) {
    settle(cx, |app| book.read(app).active_tab().is_some_and(|tab| tab.extra.kept == want));
}

#[test]
fn the_banner_follows_the_cells_showing_the_files_results() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    // A copy: the kernel locks what it opens.
    let path = dir.path().join("kept.xlsx");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/table/kept.xlsx");
    std::fs::copy(fixture, &path).expect("copies");
    update(&mut cx, &book, |book, cx| book.open_file(path.clone(), cx));
    kept(&cx, &book, &["Sheet1!B1", "Sheet1!B2", "Sheet1!B3", "Sheet1!D1", "Sheet1!E1"]);
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
