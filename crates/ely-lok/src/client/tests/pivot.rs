//! §7: an Excel pivot keeps Excel's words through a save.

use std::path::Path;

use super::independent::read_with;
use super::*;

/// Saves `name` twice, through Ely; each reads as `case`.
fn round_trip(name: &str, case: &str) {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/table").join(name);
    let doc = worker.load(&crate::file_url(&fixture)).expect("loads");
    let first = dir.path().join("first.xlsx");
    doc.save_as(&mut worker, &crate::file_url(&first), "xlsx").expect("saves");
    read_with("ooxml_pivot.py", &first, case);
    // Ely's own file reads back the same.
    let again = worker.load(&crate::file_url(&first)).expect("reloads");
    let second = dir.path().join("second.xlsx");
    again.save_as(&mut worker, &crate::file_url(&second), "xlsx").expect("saves again");
    read_with("ooxml_pivot.py", &second, case);
}

#[test]
fn a_pivot_keeps_excel_s_default_words() {
    round_trip("pivot.xlsx", "pivot");
}

#[test]
fn a_pivot_keeps_the_captions_its_file_names() {
    round_trip("pivot-named.xlsx", "pivot-named");
}

#[test]
fn a_pivot_keeps_blank_captions_blank() {
    round_trip("pivot-blank.xlsx", "pivot-blank");
}

#[test]
fn a_pivot_with_only_columns_keeps_excel_s_layout() {
    round_trip("pivot-columns.xlsx", "pivot-columns");
}

#[test]
fn values_on_rows_keep_the_column_caption() {
    round_trip("pivot-values.xlsx", "pivot-values");
}

#[test]
fn values_on_rows_keep_the_file_s_data_caption() {
    round_trip("pivot-values-named.xlsx", "pivot-values-named");
}

#[test]
fn a_pivot_with_no_values_leaves_its_neighbour() {
    round_trip("pivot-empty.xlsx", "pivot-empty");
}

#[test]
fn value_names_that_look_like_input_stay_text() {
    for name in ["pivot-name-formula.xlsx", "pivot-name-quote.xlsx", "pivot-name-number.xlsx"] {
        round_trip(name, "pivot-name");
    }
}
