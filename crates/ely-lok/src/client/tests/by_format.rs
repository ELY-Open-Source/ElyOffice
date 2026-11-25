use super::sheet::{SHEETS, open_book};
use super::*;
use crate::sheet::{self, range, structure::{self, Refused}};

/// How long a command may take to report.
const SETTLE: std::time::Duration = std::time::Duration::from_secs(3);

/// A number code Calc refuses.
const INVALID: &str = "0E";

/// Every format and two edges, as ElyTable sends them.
const LINES: &str = "align=center\nbackground=16773836\nbold=1\nborder-bottom=3:30:0\nborder-left=0:15:255\ncolor=16711680\nfont=DejaVu Sans\nitalic=1\nnumber=0.00\nsize=240\nunderline=1\nvalign=middle\nwrap=1\n";

fn check(worker: &mut Worker, doc: &Doc, address: &str, value: &str) -> Result<String, Refused> {
    structure::checked(&worker.values(doc, &structure::check_query("range.format", address, value)).expect("checked")).expect("readable")
}

fn text(worker: &mut Worker, doc: &Doc, address: &str) -> String {
    let read = range::read(&worker.values(doc, &range::read_query(address)).expect("read")).expect("readable");
    read.expect("a range").rows[0][0].text.clone()
}

/// The saved automatic styles, where cell formats live.
fn looks(dir: &std::path::Path, worker: &mut Worker, doc: &Doc) -> String {
    let out = dir.join("looks.fods");
    doc.save_as(worker, &crate::file_url(&out), "fods").expect("the save");
    let xml = std::fs::read_to_string(&out).expect("the saved book");
    let start = xml.find("<office:automatic-styles>").expect("automatic styles");
    let end = xml[start..].find("</office:automatic-styles>").expect("their end");
    xml[start..start + end].to_owned()
}

#[test]
fn a_range_takes_every_format_and_its_borders_in_one_undo_step() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_book(SHEETS);
    let base = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
    worker.post_key(&doc, 0, 1027).expect("Right to B1");
    let b1 = poll_until(&mut worker, &doc, &base, HANG, |seen| seen.cell.address == "B1");
    let shown = |worker: &mut Worker| {
        let read = worker.values(&doc, sheet::SELECTED).expect("the selection");
        sheet::selected_range(&read).expect("readable").expect("one range").range
    };
    // Protected before the undo below shows another sheet.
    worker.run(&doc, "sheet.protect", &[&sheet::protect_json("pw", &[])]).expect("protects S");
    let selected = shown(&mut worker);
    let rows = vec![vec!["3.14159".to_owned(), "2".to_owned()], vec!["7".to_owned(), "8".to_owned()]];
    worker.run(&doc, "range.set", &["U!B2:C3", &range::rows_json(&rows)]).expect("four numbers");

    assert_eq!(check(&mut worker, &doc, "U!B2:C3", LINES).map(|_| ()), Ok(()), "a sheet not shown takes formats");
    worker.run(&doc, "structure", &["range.format", "U!B2:C3", LINES]).expect("formatted");
    let after = poll_until(&mut worker, &doc, &b1, SETTLE, |_| false);
    assert_eq!(after.cell.address, "B1", "the cursor stays");
    assert_eq!(shown(&mut worker), selected, "the selection and shown sheet stay");
    assert_eq!(text(&mut worker, &doc, "U!B2"), "3.14", "the number format shows");
    let formatted = looks(dir.path(), &mut worker, &doc);
    let made = [
        "fo:font-weight=\"bold\"", "style:font-weight-asian=\"bold\"", "fo:font-style=\"italic\"", "style:text-underline-style=\"solid\"",
        "fo:color=\"#ff0000\"", "fo:background-color=\"#fff2cc\"", "fo:font-size=\"12pt\"", "DejaVu Sans", "fo:wrap-option=\"wrap\"",
        "fo:text-align=\"center\"", "style:vertical-align=\"middle\"", "#0000ff", "double",
    ];
    for look in made {
        assert!(formatted.contains(look), "{look} in {formatted}");
    }

    worker.run(&doc, "edit.undo", &[]).expect("undone");
    assert_eq!(text(&mut worker, &doc, "U!B2"), "3.14159", "one step takes the number format back");
    let undone = looks(dir.path(), &mut worker, &doc);
    for look in made {
        assert!(!undone.contains(look), "{look} is undone: {undone}");
    }

    // Calc keeps it as [RED]0.0: found again, not refused.
    worker.run(&doc, "structure", &["range.format", "U!C2", "number=[Red]0.0\n"]).expect("a code Calc respells");
    assert_eq!(text(&mut worker, &doc, "U!C2"), "2.0");
    assert_eq!(check(&mut worker, &doc, "U!B2", &format!("number={INVALID}\n")), Err(Refused::Number));
    assert_eq!(check(&mut worker, &doc, "Nope!A1", "bold=1\n"), Err(Refused::Sheet("Nope".into())));
    assert_eq!(check(&mut worker, &doc, "S!A1:", "bold=1\n"), Err(Refused::Address));
    assert_eq!(check(&mut worker, &doc, "S!A1", "bold=1\n"), Err(Refused::Protected("S".into())));
    assert!(worker.run(&doc, "structure", &["range.format", "S!A1", "bold=1\n"]).is_err(), "the kernel checks it too");
}
