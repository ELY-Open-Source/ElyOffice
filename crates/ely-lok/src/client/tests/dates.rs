//! §9.3: a 1904 workbook keeps its date system.

use super::independent::read_back;
use super::kept::{cells, open_fixture};
use super::*;

#[test]
fn a_1904_workbook_keeps_its_dates_through_saves() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let doc = open_fixture(&mut worker, "date1904.xlsx");
    // In 1900 dates these read 2020-02-29 and 45352.
    let loaded = [
        ("A1", "43890", "43890"),
        ("B1", "2024-03-01", "=TEXT(A1,\"yyyy-mm-dd\")"),
        ("C1", "03/01/24", "=DATE(2024,3,1)"),
    ];
    cells(&mut worker, &doc, &loaded);
    worker.run(&doc, "cell.set", &["Sheet1!E1", "2024-03-01"]).expect("E1");
    worker.run(&doc, "cell.set", &["Sheet1!F1", "=E1=C1"]).expect("F1");
    let typed = [("F1", "TRUE", "=E1=C1")];
    cells(&mut worker, &doc, &typed);
    let (xlsx, ods) = (dir.path().join("dates.xlsx"), dir.path().join("dates.ods"));
    doc.save_as(&mut worker, &crate::file_url(&xlsx), "xlsx").expect("saves");
    read_back(&xlsx, "dates1904");
    doc.save_as(&mut worker, &crate::file_url(&ods), "ods").expect("saves");
    worker.close_doc(doc).expect("closes");
    for copy in [&xlsx, &ods] {
        let again = worker.load(&crate::file_url(copy)).expect("reopens");
        cells(&mut worker, &again, &loaded);
        cells(&mut worker, &again, &typed);
        worker.close_doc(again).expect("closes");
    }
}
