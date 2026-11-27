//! #146: xlsx column widths survive saves in a warm worker.

use super::*;

const WIDTHS: [&str; 6] = ["20.14", "52.89", "55.13", "9.42", "90.05", "10.33"];

fn workbook() -> Vec<u8> {
    let head = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#;
    let main = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let rels = "http://schemas.openxmlformats.org/package/2006/relationships";
    let doc = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let cols: String = WIDTHS
        .iter()
        .enumerate()
        .map(|(i, w)| format!(r#"<col min="{n}" max="{n}" width="{w}" customWidth="1"/>"#, n = i + 1))
        .collect();
    let rows: String = (1..=3)
        .map(|r| format!(r#"<row r="{r}"><c r="A{r}"><v>{r}</v></c><c r="B{r}" t="inlineStr"><is><t>t{r}</t></is></c></row>"#))
        .collect();
    let parts = [
        (
            "[Content_Types].xml",
            format!(
                r#"{head}<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/></Types>"#
            ),
        ),
        (
            "_rels/.rels",
            format!(r#"{head}<Relationships xmlns="{rels}"><Relationship Id="rId1" Type="{doc}/officeDocument" Target="xl/workbook.xml"/></Relationships>"#),
        ),
        (
            "xl/workbook.xml",
            format!(r#"{head}<workbook xmlns="{main}" xmlns:r="{doc}"><sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets></workbook>"#),
        ),
        (
            "xl/_rels/workbook.xml.rels",
            format!(
                r#"{head}<Relationships xmlns="{rels}"><Relationship Id="rId1" Type="{doc}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{doc}/styles" Target="styles.xml"/></Relationships>"#
            ),
        ),
        (
            "xl/styles.xml",
            format!(
                r#"{head}<styleSheet xmlns="{main}"><fonts count="1"><font><sz val="11"/><name val="Calibri"/><family val="2"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border/></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>"#
            ),
        ),
        (
            "xl/worksheets/sheet1.xml",
            format!(r#"{head}<worksheet xmlns="{main}"><cols>{cols}</cols><sheetData>{rows}</sheetData></worksheet>"#),
        ),
    ];
    let entries: Vec<(String, Vec<u8>)> = parts
        .into_iter()
        .map(|(name, body)| (name.to_string(), body.into_bytes()))
        .collect();
    super::zip::stored(&entries)
}

fn attribute<'a>(element: &'a str, name: &str) -> &'a str {
    let key = format!(" {name}=\"");
    let start = element.find(&key).unwrap_or_else(|| panic!("no {name} in {element}")) + key.len();
    let end = element[start..].find('"').expect("a closed value");
    &element[start..start + end]
}

/// Each column's width, from the first column on.
fn widths(file: &std::path::Path) -> Vec<String> {
    let bytes = std::fs::read(file).expect("reads the save");
    let parts = super::zip::entries(&bytes);
    let (_, sheet) = parts
        .iter()
        .find(|(name, _)| name == "xl/worksheets/sheet1.xml")
        .expect("a sheet");
    let sheet = String::from_utf8_lossy(sheet);
    let mut found: Vec<(usize, String)> = sheet
        .split("<col ")
        .skip(1)
        .map(|rest| {
            let element = &rest[..rest.find('>').expect("a closed col")];
            let element = format!(" {element}");
            let min = attribute(&element, "min").parse().expect("a column number");
            (min, attribute(&element, "width").to_string())
        })
        .collect();
    found.sort();
    found.into_iter().map(|(_, width)| width).collect()
}

#[test]
fn xlsx_column_widths_survive_two_saves_in_a_warm_worker() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    // The resident engine has always rendered a document first.
    let _blank = worker.load("private:factory/scalc").expect("a blank workbook");
    let source = dir.path().join("source.xlsx");
    std::fs::write(&source, workbook()).expect("writes the workbook");
    let mut from = source;
    for round in 1..=2 {
        let doc = worker.load(&crate::file_url(&from)).expect("loads");
        let to = dir.path().join(format!("save{round}.xlsx"));
        doc.save_as(&mut worker, &crate::file_url(&to), "xlsx").expect("saves");
        assert_eq!(widths(&to), WIDTHS, "widths after save {round}");
        from = to;
    }
}
