//! Merge data from spreadsheets (GitHub #67).

use gpui::TestAppContext;

use super::super::run::read_sheet;
use crate::tests::{blank, docs, settle};

const FODS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="People">
  <table:table-row><table:table-cell office:value-type="string"><text:p>Name</text:p></table:table-cell><table:table-cell office:value-type="string"><text:p>City</text:p></table:table-cell><table:table-cell office:value-type="string"><text:p>Age</text:p></table:table-cell></table:table-row>
  <table:table-row><table:table-cell office:value-type="string"><text:p>Ada</text:p></table:table-cell><table:table-cell office:value-type="string"><text:p>上海</text:p></table:table-cell><table:table-cell office:value-type="float" office:value="36"><text:p>36</text:p></table:table-cell></table:table-row>
 </table:table></office:spreadsheet></office:body>
</office:document>"#;

#[test]
fn spreadsheets_read_as_merge_data() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &docs);
    let flat = dir.path().join("people.fods");
    std::fs::write(&flat, FODS).expect("the sheet writes");
    let worker = cx.update(|app| docs.read(app).core.worker.clone());
    let formats = ["xlsx", "xls", "ods"];
    let sheets: Vec<_> = formats.iter().map(|format| dir.path().join(format!("people.{format}"))).collect();
    {
        let mut held = worker.lock().expect("the worker lock");
        let doc = held.load(&ely_lok::file_url(&flat)).expect("the sheet loads");
        for (sheet, format) in sheets.iter().zip(formats) {
            doc.save_as(&mut held, &ely_lok::file_url(sheet), format).expect("the sheet saves");
        }
        held.close_doc(doc).expect("closes");
    }
    for sheet in sheets {
        let name = sheet.file_name().expect("a name").to_string_lossy().to_string();
        cx.update(|app| docs.update(app, |docs, cx| docs.choose_csv(cx)));
        cx.simulate_path_prompt_response(move |_| Some(vec![sheet]));
        settle(&cx, |app| docs.read(app).merge.read(app).table.as_ref().is_some_and(|table| table.name == name.as_str()));
        let table = cx.update(|app| docs.read(app).merge.read(app).table.clone().expect("the data"));
        assert_eq!(table.headers, ["Name", "City", "Age"], "{name}");
        assert_eq!(table.rows, [["Ada", "上海", "36"]], "{name}");
    }
    let bad = dir.path().join("bad.xlsx");
    std::fs::write(&bad, "not a workbook").expect("writes");
    let scratch = dir.path().join("scratch.csv");
    assert!(read_sheet(&worker, &bad, &scratch).is_err(), "a broken workbook fails");
    assert!(!scratch.exists(), "no scratch is left");
}
