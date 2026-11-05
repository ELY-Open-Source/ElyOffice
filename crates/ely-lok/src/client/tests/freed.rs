//! LOK-11: sources read for links stay free to replace.

use std::path::Path;

use super::analysis::{cells, skip};
use super::linked::{includes, rates, update};
use super::ole_links::worksheet;
use super::*;

fn fresh() -> (tempfile::TempDir, Worker) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let install = install().expect("the pinned LOK build");
    let worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    (dir, worker)
}

/// Renames a copy over `source`, as editors save.
fn replace(source: &Path, when: &str) {
    let copy = source.with_extension("copy");
    std::fs::copy(source, &copy).expect("copies the source");
    if let Err(error) = std::fs::rename(&copy, source) {
        panic!("{when}: the source is held: {error}");
    }
}

#[test]
fn a_source_a_book_read_is_free_to_replace() {
    if skip() {
        return;
    }
    let (dir, mut worker) = fresh();
    let source = dir.path().join("source.xlsx");
    worksheet(dir.path(), &mut worker, &source, 1);
    let flat = dir.path().join("reader.fods");
    rates(&flat, 0);
    let reader = worker.load(&crate::file_url(&flat)).expect("loads");
    let formula = format!("='{}/[source.xlsx]S1'!A1", dir.path().display());
    worker.run(&reader, "cell.goto", &["H1"]).expect("H1");
    worker.run(&reader, "cell.enter", &[&formula]).expect("the link");
    assert_eq!(cells(&mut worker, &reader, "H1"), "1");
    replace(&source, "after a formula read it");
    let saved = dir.path().join("reader.xlsx");
    reader.save_as(&mut worker, &crate::file_url(&saved), "xlsx").expect("saves");
    worker.close_doc(reader).expect("closes");
    worksheet(dir.path(), &mut worker, &source, 2);
    let book = worker.load(&crate::file_url(&saved)).expect("reloads");
    assert_eq!(update(&mut worker, &book), Vec::<String>::new());
    assert_eq!(cells(&mut worker, &book, "H1"), "2");
    replace(&source, "after an update read it");
    // Saved over in place, as ElyTable saves it.
    worksheet(dir.path(), &mut worker, &source, 3);
    assert_eq!(update(&mut worker, &book), Vec::<String>::new());
    assert_eq!(cells(&mut worker, &book, "H1"), "3", "the next update reads the new file");
}

/// S1.A1 is 1; four big sheets slow the import.
fn slow_source(path: &Path) {
    let script = r#"import sys, zipfile
ns = 'xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"'
rel = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships'
names = ['S1', 'F1', 'F2', 'F3', 'F4']
with zipfile.ZipFile(sys.argv[1], 'w', zipfile.ZIP_DEFLATED) as z:
    z.writestr('[Content_Types].xml', '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>' + ''.join(f'<Override PartName="/xl/worksheets/sheet{i}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>' for i in range(len(names))) + '</Types>')
    z.writestr('_rels/.rels', f'<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="r1" Type="{rel}/officeDocument" Target="xl/workbook.xml"/></Relationships>')
    z.writestr('xl/workbook.xml', f'<workbook {ns} xmlns:r="{rel}"><sheets>' + ''.join(f'<sheet name="{n}" sheetId="{i + 1}" r:id="r{i}"/>' for i, n in enumerate(names)) + '</sheets></workbook>')
    z.writestr('xl/_rels/workbook.xml.rels', '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">' + ''.join(f'<Relationship Id="r{i}" Type="{rel}/worksheet" Target="worksheets/sheet{i}.xml"/>' for i in range(len(names))) + '</Relationships>')
    z.writestr('xl/worksheets/sheet0.xml', f'<worksheet {ns}><sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData></worksheet>')
    for i in range(1, len(names)):
        rows = ''.join(f'<row r="{r}"><c r="A{r}"><v>{r}.{i}</v></c></row>' for r in range(1, 400001))
        z.writestr(f'xl/worksheets/sheet{i}.xml', f'<worksheet {ns}><sheetData>{rows}</sheetData></worksheet>')
"#;
    let out = std::process::Command::new("python3").args(["-c", script]).arg(path).output().expect("python3 runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn a_read_while_the_source_imports_gets_its_value() {
    if skip() {
        return;
    }
    let (dir, mut worker) = fresh();
    slow_source(&dir.path().join("source.xlsx"));
    let flat = dir.path().join("reader.fods");
    rates(&flat, 0);
    let formula = format!("='{}/[source.xlsx]S1'!A1", dir.path().display());
    let reader = worker.load(&crate::file_url(&flat)).expect("loads");
    let loaded = std::time::Instant::now();
    worker.run(&reader, "cell.goto", &["H1"]).expect("H1");
    worker.run(&reader, "cell.enter", &[&formula]).expect("the link");
    // Kernel idle imports the source ~0.6 s after load.
    std::thread::sleep(std::time::Duration::from_secs(1).saturating_sub(loaded.elapsed()));
    assert_eq!(cells(&mut worker, &reader, "H1"), "1", "a read ran inside the kernel's import");
}

#[test]
fn a_source_a_document_read_is_free_to_replace() {
    if skip() {
        return;
    }
    let (dir, mut worker) = fresh();
    let (main, include) = includes(&dir, &mut worker, "<text:p>One</text:p>");
    let doc = worker.load(&crate::file_url(&main)).expect("loads");
    assert_eq!(update(&mut worker, &doc), Vec::<String>::new());
    replace(&include, "after an update read the INCLUDETEXT source");
    worker.close_doc(doc).expect("closes");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office/ole-link.docx");
    let (main, source) = (dir.path().join("ole.docx"), dir.path().join("source.xlsx"));
    std::fs::copy(fixture, &main).expect("copies");
    worksheet(dir.path(), &mut worker, &source, 4);
    let doc = worker.load(&crate::file_url(&main)).expect("loads");
    assert_eq!(update(&mut worker, &doc), Vec::<String>::new());
    replace(&source, "after an update read the linked object");
}
