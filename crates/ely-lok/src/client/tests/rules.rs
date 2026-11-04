use super::sheet::open_book;
use super::*;
use crate::sheet;

pub(super) fn row(cells: &[&str]) -> String {
    let cells: String = cells
        .iter()
        .map(|cell| match cell.parse::<f64>() {
            Ok(_) => format!(
                r#"<table:table-cell office:value-type="float" office:value="{cell}"><text:p>{cell}</text:p></table:table-cell>"#
            ),
            Err(_) => format!(r#"<table:table-cell><text:p>{cell}</text:p></table:table-cell>"#),
        })
        .collect();
    format!("<table:table-row>{cells}</table:table-row>")
}

/// Name, Qty, City; one repeated row.
fn data() -> String {
    let rows: String = [
        ["Name", "Qty", "City"],
        ["a", "3", "x"],
        ["b", "1", "y"],
        ["a", "3", "x"],
        ["c", "2", "y"],
    ]
    .iter()
    .map(|cells| row(cells))
    .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="S">{rows}</table:table></office:spreadsheet></office:body>
</office:document>"#
    )
}

fn open() -> (tempfile::TempDir, Worker, Doc, Events) {
    let (dir, mut worker, doc) = open_book(&data());
    let seen = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
        seen.cell.cursor.is_some()
    });
    (dir, worker, doc, seen)
}

/// The cells of `range`, as the kernel copies them.
fn cells(worker: &mut Worker, doc: &Doc, range: &str) -> String {
    worker.run(doc, "cell.goto", &[range]).expect("the range");
    // Windows ends copied lines with CRLF.
    doc.selection_text(worker).expect("the text").replace("\r\n", "\n")
}

/// The latest report of `name`.
fn state<'a>(seen: &'a Events, name: &str) -> Option<&'a str> {
    seen.states
        .iter()
        .rfind(|(state, _)| state == name)
        .map(|(_, value)| value.as_str())
}

fn skip() -> bool {
    let missing = install().is_none();
    if missing {
        eprintln!("SKIP: no pinned LOK build");
    }
    missing
}

#[test]
fn number_formats_alignment_and_fills_apply() {
    if skip() {
        return;
    }
    let (_dir, mut worker, doc, seen) = open();
    worker.run(&doc, "cell.goto", &["B2:B5"]).expect("B2:B5");
    let read = worker.values(&doc, sheet::NUMBER).expect("the format");
    let kind = sheet::number_format(&read).expect("parses").1;
    assert_eq!(kind, "general", "the standard format is general");
    worker
        .run(&doc, "number.code", &["0.00%"])
        .expect("a format code");
    let read = worker.values(&doc, sheet::NUMBER).expect("the format");
    assert_eq!(
        sheet::number_format(&read).expect("parses"),
        ("0.00%".to_string(), "percent".to_string())
    );
    worker.run(&doc, "number.currency", &[]).expect("currency");
    let read = worker.values(&doc, sheet::NUMBER).expect("the format");
    assert_eq!(sheet::number_format(&read).expect("parses").1, "currency");
    let check = |worker: &mut Worker, code: &str| {
        let json = worker
            .values(&doc, &sheet::number_check_query(code))
            .expect("the check");
        sheet::number_valid(&json).expect("parses")
    };
    assert!(check(&mut worker, "#,##0.00 \"kg\""));
    assert!(!check(&mut worker, "[ZZZ]0"), "an unknown keyword fails");
    assert!(
        worker.run(&doc, "number.code", &["[ZZZ]0"]).is_err(),
        "the kernel refuses a broken code"
    );
    worker.run(&doc, "align.center", &[]).expect("center");
    worker.run(&doc, "cells.wrap", &["true"]).expect("wrap");
    let states = poll_until(&mut worker, &doc, &seen, HANG, |seen| {
        state(seen, ".uno:AlignHorizontalCenter") == Some("true")
            && state(seen, ".uno:WrapText") == Some("true")
    });
    assert_eq!(state(&states, ".uno:WrapText"), Some("true"));
    // The cell properties of the document's own cell formats.
    let saved = |worker: &mut Worker| {
        let out = _dir.path().join("borders.fods");
        doc.save_as(worker, &crate::file_url(&out), "fods")
            .expect("the save");
        let xml = std::fs::read_to_string(&out).expect("the saved book");
        let start = xml.find("<office:automatic-styles>").expect("cell formats");
        xml[start..]
            .split("<style:table-cell-properties")
            .skip(1)
            .map(|cell| cell.split("/>").next().expect("a closed element"))
            .collect::<String>()
    };
    worker.run(&doc, "cells.borders_all", &[]).expect("all");
    let all = saved(&mut worker);
    assert!(
        all.contains("solid") && !all.contains("fo:border-top=\"none\""),
        "every line"
    );
    worker.run(&doc, "cells.borders_outer", &[]).expect("outer");
    let outer = saved(&mut worker);
    assert!(
        outer.contains("solid") && outer.contains("fo:border-top=\"none\""),
        "no inner lines"
    );
    worker.run(&doc, "cells.borders_none", &[]).expect("none");
    assert!(!saved(&mut worker).contains("solid"), "no lines");
    worker.run(&doc, "cells.fill", &["16711680"]).expect("red");
    assert!(saved(&mut worker).contains("fo:background-color=\"#ff0000\""));
    worker.run(&doc, "cells.fill", &["-1"]).expect("no fill");
    assert!(!saved(&mut worker).contains("#ff0000"), "the fill clears");
}

#[test]
fn sorting_filtering_and_duplicates() {
    if skip() {
        return;
    }
    let (_dir, mut worker, doc, _seen) = open();
    worker.run(&doc, "cell.goto", &["A1"]).expect("A1");
    let sort = ["true", "2", "true", "0", "true", "0", "true"];
    worker.run(&doc, "data.sort", &sort).expect("by Qty");
    assert_eq!(cells(&mut worker, &doc, "A2:A5").trim(), "b\nc\na\na");
    worker.run(&doc, "cell.goto", &["A1"]).expect("A1");
    let query = r#"{"conditions":[{"field":2,"op":"equals","value":"x"}]}"#;
    worker
        .run(&doc, "data.filter", &[query])
        .expect("City is x");
    let dropped = |worker: &mut Worker, column: u32| -> Vec<String> {
        let query = sheet::cell_query(sheet::FILTER, column, 0);
        let json = worker.values(&doc, &query).expect("the entries");
        sheet::filter_values(&json)
            .expect("parses")
            .entries
            .into_iter()
            .filter(|entry| !entry.selected)
            .map(|entry| entry.text)
            .collect()
    };
    assert_eq!(dropped(&mut worker, 2), ["y"]);
    let picked = r#"{"column":0,"conditions":[{"op":"values","values":["c"]}]}"#;
    worker
        .run(&doc, "data.filter", &[picked])
        .expect("Name is c");
    assert_eq!(dropped(&mut worker, 0), ["a", "b"], "the column keeps c");
    assert_eq!(dropped(&mut worker, 2), ["y"], "the City filter stays");
    worker.run(&doc, "data.unfilter", &[]).expect("no filter");
    assert!(dropped(&mut worker, 2).is_empty());
    let bad = r#"{"conditions":[{"field":9,"op":"equals","value":"x"}]}"#;
    assert!(
        worker.run(&doc, "data.filter", &[bad]).is_err(),
        "out of the data"
    );

    worker.run(&doc, "cell.goto", &["B3"]).expect("B3");
    worker.run(&doc, "select.data", &[]).expect("the data area");
    let json = worker.values(&doc, sheet::SELECTED).expect("the selection");
    let picked = sheet::selected_range(&json)
        .expect("parses")
        .expect("one range");
    assert_eq!(
        (picked.top.as_slice(), picked.header),
        (&["Name".to_string(), "Qty".into(), "City".into()][..], true)
    );
    let (c1, r1, c2, r2) = picked.area.expect("data");
    assert_eq!((c1, r1, c2, r2), (0, 0, 2, 4), "{}", picked.range);
    let corners = [c1, r1, c2, r2].map(|n| n.to_string());
    let mut values = vec!["true", "true", "true"];
    values.extend(corners.iter().map(String::as_str));
    values.push("0");
    let now = worker.now(&doc).expect("the state");
    worker.run(&now, "data.dedupe", &values).expect("removes");
    assert_eq!(cells(&mut worker, &doc, "A2:A5").trim(), "b\nc\na");
}

#[test]
fn a_filter_button_and_alt_down_ask_for_ely_menus() {
    if skip() {
        return;
    }
    let (_dir, mut worker, doc, seen) = open();
    worker.run(&doc, "cell.goto", &["A1"]).expect("A1");
    worker.run(&doc, "data.autofilter", &[]).expect("buttons");
    let a1 = seen.cell.cursor.expect("the cursor");
    click(&mut worker, &doc, a1.0 + a1.2 - 60, a1.1 + a1.3 / 2);
    let asked = poll_until(&mut worker, &doc, &seen, HANG, |seen| {
        seen.cell.request.is_some()
    });
    assert_eq!(
        asked.cell.request,
        Some(Request::Filter { column: 0, row: 0 })
    );
    worker.run(&doc, "cell.goto", &["E2"]).expect("E2");
    let rule = crate::sheet::Validation {
        kind: "list".into(),
        operator: "equal".into(),
        list: vec!["Red".into(), "Green \"G\"".into()],
        dropdown: true,
        ignore_blank: true,
        show_input: true,
        input_title: "Pick".into(),
        input_message: "A color".into(),
        show_error: true,
        error_style: "stop".into(),
        error_title: "Bad".into(),
        error_message: "Red or green".into(),
        ..Default::default()
    };
    worker
        .run(&doc, "rules.validation", &[&rule.to_json()])
        .expect("a list");
    let json = worker
        .values(&doc, &sheet::cell_query(sheet::LIST, 4, 1))
        .expect("the choices");
    assert_eq!(
        sheet::list_values(&json).expect("parses"),
        ["Red", "Green \"G\""]
    );
    let read = sheet::validation(&worker.values(&doc, sheet::VALIDATION).expect("the rule"))
        .expect("parses");
    assert_eq!(
        (
            read.kind.as_str(),
            read.dropdown,
            read.input_message.as_str()
        ),
        ("list", true, "A color")
    );
    worker.post_key(&doc, 0, 1024 | 0x4000).expect("Alt+Down");
    let listed = poll_until(&mut worker, &doc, &asked, HANG, |seen| {
        seen.cell.requests > asked.cell.requests
    });
    assert_eq!(
        listed.cell.request,
        Some(Request::Choices { column: 4, row: 1 })
    );
    for c in "Blue".chars() {
        worker.post_key(&doc, c as u32, 0).expect("a key");
    }
    worker.post_key(&doc, 13, 1280).expect("Enter");
    let refused = poll_until(&mut worker, &doc, &listed, HANG, |seen| {
        seen.cell.requests > listed.cell.requests
    });
    assert!(
        matches!(
            &refused.cell.request,
            Some(Request::Invalid { accepted: false, title, .. }) if title == "Bad"
        ),
        "{:?}",
        refused.cell.request
    );
    worker.run(&doc, "cell.goto", &["E3"]).expect("E3");
    let away = poll_until(&mut worker, &doc, &refused, HANG, |seen| {
        seen.cell.address == "E3"
    });
    worker.run(&doc, "cell.goto", &["E2"]).expect("E2");
    let kept = poll_until(&mut worker, &doc, &away, HANG, |seen| {
        seen.cell.address == "E2"
    });
    assert_eq!(kept.cell.formula, "", "a stop keeps it out");
}

#[test]
fn any_value_keeps_its_input_help() {
    if skip() {
        return;
    }
    let (_dir, mut worker, doc, _seen) = open();
    worker.run(&doc, "cell.goto", &["D2"]).expect("D2");
    let rule = crate::sheet::Validation {
        kind: "any".into(),
        operator: "between".into(),
        show_input: true,
        input_message: "An id".into(),
        error_style: "stop".into(),
        ..Default::default()
    };
    worker
        .run(&doc, "rules.validation", &[&rule.to_json()])
        .expect("help alone");
    let read = sheet::validation(&worker.values(&doc, sheet::VALIDATION).expect("the rule"))
        .expect("parses");
    assert_eq!(
        (read.show_input, read.input_message.as_str()),
        (true, "An id")
    );
    let none = crate::sheet::Validation {
        kind: "any".into(),
        operator: "between".into(),
        error_style: "stop".into(),
        ..Default::default()
    };
    worker
        .run(&doc, "rules.validation", &[&none.to_json()])
        .expect("cleared");
    let read = sheet::validation(&worker.values(&doc, sheet::VALIDATION).expect("the rule"))
        .expect("parses");
    assert!(!read.show_input, "the help is gone");
}

#[test]
fn whole_numbers_text_columns_and_value_pastes() {
    if skip() {
        return;
    }
    let (_dir, mut worker, doc, seen) = open();
    worker.run(&doc, "cell.goto", &["B2:B5"]).expect("B2:B5");
    let rule = crate::sheet::Validation {
        kind: "whole".into(),
        operator: "between".into(),
        value1: "1".into(),
        value2: "10".into(),
        ignore_blank: true,
        show_error: true,
        error_style: "info".into(),
        ..Default::default()
    };
    worker
        .run(&doc, "rules.validation", &[&rule.to_json()])
        .expect("whole numbers");
    worker.run(&doc, "cell.goto", &["B2"]).expect("B2");
    for c in "50".chars() {
        worker.post_key(&doc, c as u32, 0).expect("a key");
    }
    worker.post_key(&doc, 13, 1280).expect("Enter");
    let warned = poll_until(&mut worker, &doc, &seen, HANG, |seen| {
        seen.cell.request.is_some()
    });
    assert!(
        matches!(
            warned.cell.request,
            Some(Request::Invalid { accepted: true, .. })
        ),
        "{:?}",
        warned.cell.request
    );
    assert_eq!(
        cells(&mut worker, &doc, "B2").trim(),
        "50",
        "a note lets it in"
    );

    worker.run(&doc, "cell.goto", &["A7"]).expect("A7");
    worker.run(&doc, "cell.enter", &["p;q;r"]).expect("text");
    worker.run(&doc, "cell.goto", &["A7"]).expect("A7");
    worker
        .run(&doc, "data.columns", &["59,34,76,1"])
        .expect("split");
    assert_eq!(cells(&mut worker, &doc, "A7:C7").trim(), "p\tq\tr");

    worker.run(&doc, "cell.goto", &["E2"]).expect("E2");
    worker
        .run(&doc, "cell.enter", &["=B3*2"])
        .expect("a formula");
    worker.run(&doc, "cell.goto", &["E2"]).expect("E2");
    worker.copy(&doc).expect("the copy");
    worker.run(&doc, "cell.goto", &["F2"]).expect("F2");
    worker.run(&doc, "paste.values", &[]).expect("values");
    worker.run(&doc, "cell.goto", &["G2"]).expect("G2");
    worker.run(&doc, "paste.formulas", &[]).expect("formulas");
    worker.run(&doc, "cell.goto", &["F2"]).expect("F2");
    let pasted = poll_until(&mut worker, &doc, &warned, HANG, |seen| {
        seen.cell.address == "F2" && !seen.cell.formula.is_empty()
    });
    assert_eq!(pasted.cell.formula, "2", "the value alone");
    worker.run(&doc, "cell.goto", &["G2"]).expect("G2");
    let formula = poll_until(&mut worker, &doc, &pasted, HANG, |seen| {
        seen.cell.address == "G2"
    });
    assert_eq!(formula.cell.formula, "=D3*2", "the reference moves along");
}

#[test]
fn conditional_formats_add_list_and_go() {
    if skip() {
        return;
    }
    let (_dir, mut worker, doc, _seen) = open();
    worker.run(&doc, "cell.goto", &["B2:B5"]).expect("B2:B5");
    let read = |worker: &mut Worker| {
        sheet::formats(&worker.values(&doc, sheet::FORMATS).expect("the formats")).expect("parses")
    };
    for rule in [
        r#"{"kind":"cell","operator":"greater","value1":"2","style":"Good"}"#,
        r##"{"kind":"scale","colors":["#F8696B","#FFEB84","#63BE7B"]}"##,
        r##"{"kind":"databar","color":"#638EC6"}"##,
    ] {
        worker.run(&doc, "rules.conditional", &[rule]).expect(rule);
    }
    let formats = read(&mut worker);
    let kinds: Vec<&str> = formats.iter().map(|format| format.kind.as_str()).collect();
    assert_eq!(kinds, ["cell", "scale", "databar"]);
    assert_eq!(
        (
            formats[0].style.as_str(),
            formats[0].value.as_str(),
            formats[0].range.as_str()
        ),
        ("Good", "2", "B2:B5")
    );
    let unknown = r#"{"kind":"cell","operator":"greater","value1":"2","style":"Nope"}"#;
    assert!(worker.run(&doc, "rules.conditional", &[unknown]).is_err());
    let remove = format!(r#"{{"remove":{}}}"#, formats[1].key);
    worker
        .run(&doc, "rules.conditional", &[&remove])
        .expect("removes");
    assert_eq!(read(&mut worker).len(), 2);
}

#[test]
fn search_counts_and_replaces_cells() {
    if skip() {
        return;
    }
    let (_dir, mut worker, doc, seen) = open();
    let find = ["a", "false", "1", "256", "0", "1"];
    worker.run(&doc, "find.run", &find).expect("find all");
    let all = poll_until(&mut worker, &doc, &seen, HANG, |seen| seen.searched > 0);
    assert_eq!(all.found.len(), 3, "one per cell: {:?}", all.found);
    let step = ["a", "false", "0", "256", "0", "1"];
    worker.run(&doc, "find.run", &step).expect("find next");
    let next = poll_until(&mut worker, &doc, &all, HANG, |seen| {
        seen.searched > all.searched
    });
    let at = next.found.first().expect("a match");
    assert!(all.found.contains(at), "{at:?} is one of {:?}", all.found);
    let replace = ["a", "false", "3", "256", "0", "1", "z"];
    worker
        .run(&doc, "find.replace", &replace)
        .expect("replace all");
    assert_eq!(cells(&mut worker, &doc, "A1:A5").trim(), "Nzme\nz\nb\nz\nc");
}

#[test]
fn enter_and_tab_move_the_cell_cursor() {
    if skip() {
        return;
    }
    let (_dir, mut worker, doc, seen) = open();
    worker.run(&doc, "cell.goto", &["F1"]).expect("F1");
    let at = poll_until(&mut worker, &doc, &seen, HANG, |seen| {
        seen.cell.address == "F1"
    });
    worker.post_text(&doc, true, "7").expect("typed text");
    worker.post_key(&doc, 0, 1280).expect("Enter");
    let down = poll_until(&mut worker, &doc, &at, HANG, |seen| {
        seen.cell.address == "F2"
    });
    assert_eq!(down.cell.address, "F2", "Enter moves down");
    worker.post_key(&doc, '8' as u32, 0).expect("a key");
    worker.post_key(&doc, 0, 1282).expect("Tab");
    let right = poll_until(&mut worker, &doc, &down, HANG, |seen| {
        seen.cell.address == "G2"
    });
    assert_eq!(right.cell.address, "G2", "Tab moves right");
}
