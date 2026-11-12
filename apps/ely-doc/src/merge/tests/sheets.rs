//! Merge data from spreadsheets (GitHub #67).

use gpui::TestAppContext;

use super::super::run::{read_in, read_sheet};
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
    let work = dir.path().join("reads");
    std::fs::create_dir(&work).expect("the work dir");
    assert!(read_sheet(&worker, &bad, &work).is_err(), "a broken workbook fails");
    let good = dir.path().join("people.xlsx");
    assert_eq!(read_sheet(&worker, &good, &work).expect("reads").rows, [["Ada", "上海", "36"]]);
    assert_eq!(std::fs::read_dir(&work).expect("reads").count(), 0, "no scratch is left");
}

#[test]
fn each_read_writes_only_in_its_own_folder() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let worker = cx.update(|app| docs.read(app).core.worker.clone());
    let work = dir.path().join("reads");
    std::fs::create_dir(&work).expect("the work dir");
    let names = ["Ada", "Bob"];
    let source = dir.path().join("sheets");
    std::fs::create_dir(&source).expect("the sheets dir");
    let sheets: Vec<_> = names.iter().map(|name| source.join(format!("{name}.xlsx"))).collect();
    for (name, sheet) in names.iter().zip(&sheets) {
        let flat = dir.path().join(format!("{name}.fods"));
        std::fs::write(&flat, FODS.replace("Ada", name)).expect("the sheet writes");
        let mut held = worker.lock().expect("the worker lock");
        let doc = held.load(&ely_lok::file_url(&flat)).expect("the sheet loads");
        doc.save_as(&mut held, &ely_lok::file_url(sheet), "xlsx").expect("the sheet saves");
        held.close_doc(doc).expect("closes");
    }
    let folders: Vec<_> = names.iter().map(|_| tempfile::tempdir_in(&work).expect("a folder")).collect();
    // No shared file beside the folders or the sheets.
    #[cfg(unix)]
    let lock = |mode| {
        for shared in [&work, &source] {
            std::fs::set_permissions(shared, std::os::unix::fs::PermissionsExt::from_mode(mode)).expect("sets the mode");
        }
    };
    #[cfg(unix)]
    lock(0o555);
    let reads: Vec<_> = sheets.iter().zip(&folders).map(|(sheet, folder)| read_in(&worker, sheet, folder.path())).collect();
    #[cfg(unix)]
    lock(0o755);
    for ((name, read), folder) in names.iter().zip(reads).zip(folders) {
        assert_eq!(read.expect("the sheet reads").rows, [[*name, "上海", "36"]], "{name}");
        folder.close().expect("the folder goes");
    }
    assert_eq!(std::fs::read_dir(&work).expect("reads").count(), 0, "nothing beside the folders");
    assert_eq!(std::fs::read_dir(&source).expect("reads").count(), 2, "nothing beside the sheets");
}

#[test]
fn the_newest_choice_wins_while_sheets_read() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let source = blank(&mut cx, &docs);
    let worker = cx.update(|app| docs.read(app).core.worker.clone());
    let flat = dir.path().join("people.fods");
    std::fs::write(&flat, FODS).expect("the sheet writes");
    let (old, older) = (dir.path().join("old.xlsx"), dir.path().join("older.xlsx"));
    {
        let mut held = worker.lock().expect("the worker lock");
        let doc = held.load(&ely_lok::file_url(&flat)).expect("the sheet loads");
        for sheet in [&old, &older] {
            doc.save_as(&mut held, &ely_lok::file_url(sheet), "xlsx").expect("the sheet saves");
        }
        held.close_doc(doc).expect("closes");
    }
    let new = dir.path().join("new.csv");
    std::fs::write(&new, "Name,City
New,Paris
").expect("the csv writes");
    cx.update(|app| {
        docs.update(app, |docs, cx| {
            for path in [&older, &old, &new] {
                docs.take_csv(source, path, cx);
            }
        })
    });
    let work = cx.update(|app| docs.read(app).core.work.clone());
    let scratch = || {
        let entries = std::fs::read_dir(&work).expect("the work dir").flatten();
        entries.filter(|entry| entry.file_name().to_string_lossy().starts_with("merge-")).count()
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    settle(&cx, |_| std::time::Instant::now() > deadline && worker.try_lock().is_ok());
    let table = cx.update(|app| docs.read(app).merge.read(app).table.clone().expect("the data"));
    assert_eq!(table.name, "new.csv", "a stale read replaced the newest choice");
    assert_eq!(table.rows, [["New", "Paris"]]);
    assert_eq!(scratch(), 0, "no scratch is left");
}
