use ely_gpui_component::tables::{FilterRule, SortKey, Test};
use ely_lok::{Request, sheet};

use super::*;

/// Name, Qty, City; one repeated row.
const DATA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="S">
 <table:table-row><table:table-cell><text:p>Name</text:p></table:table-cell><table:table-cell><text:p>Qty</text:p></table:table-cell><table:table-cell><text:p>City</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell><text:p>a</text:p></table:table-cell><table:table-cell office:value-type="float" office:value="3"><text:p>3</text:p></table:table-cell><table:table-cell><text:p>x</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell><text:p>b</text:p></table:table-cell><table:table-cell office:value-type="float" office:value="1"><text:p>1</text:p></table:table-cell><table:table-cell><text:p>y</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell><text:p>a</text:p></table:table-cell><table:table-cell office:value-type="float" office:value="3"><text:p>3</text:p></table:table-cell><table:table-cell><text:p>x</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell><text:p>c</text:p></table:table-cell><table:table-cell office:value-type="float" office:value="2"><text:p>2</text:p></table:table-cell><table:table-cell><text:p>y</text:p></table:table-cell></table:table-row>
 </table:table></office:spreadsheet></office:body>
</office:document>"#;

pub(super) fn open(cx: &mut TestAppContext, book: &Entity<Book>) {
    update(cx, book, |book, cx| {
        book.new_from_text("data.fods", DATA, cx)
    });
    settle(cx, |app| sheet_names(app, book) == ["S"]);
}

/// Waits for the active tab's queued input.
pub(super) fn quiet(cx: &TestAppContext, book: &Entity<Book>) {
    settle(cx, |app| {
        book.read(app)
            .active_tab()
            .is_some_and(|tab| tab.view.read(app).settled())
    });
}

/// The cells of `range`, read through the worker.
pub(super) fn cells(cx: &mut TestAppContext, book: &Entity<Book>, range: &str) -> String {
    quiet(cx, book);
    cx.update(|app| {
        let book = book.read(app);
        let tab = book.active_tab().expect("a tab");
        let mut worker = book.core.worker.lock().expect("the worker lock");
        worker
            .run(&tab.doc, "cell.goto", &[range])
            .expect("the range");
        let text = tab.doc.selection_text(&mut worker).expect("the text");
        // The Windows kernel ends rows with CRLF.
        text.trim().lines().collect::<Vec<_>>().join("\n")
    })
}

pub(super) fn run(cx: &mut TestAppContext, book: &Entity<Book>, id: &str, values: &[&str]) {
    update(cx, book, |book, cx| book.run_active(id, values, cx));
    quiet(cx, book);
}

/// The column's entries the filter drops.
fn dropped(cx: &mut TestAppContext, book: &Entity<Book>, column: u32) -> Vec<String> {
    quiet(cx, book);
    cx.update(|app| {
        let book = book.read(app);
        let tab = book.active_tab().expect("a tab");
        let mut worker = book.core.worker.lock().expect("the worker lock");
        let json = worker
            .values(&tab.doc, &sheet::cell_query(sheet::FILTER, column, 0))
            .expect("the entries");
        sheet::filter_values(&json)
            .expect("parses")
            .entries
            .into_iter()
            .filter(|entry| !entry.selected)
            .map(|entry| entry.text)
            .collect()
    })
}

pub(super) fn asked(cx: &TestAppContext, book: &Entity<Book>, is: fn(&Dialog) -> bool) {
    settle(cx, |app| book.read(app).dialog.as_ref().is_some_and(is));
}

#[test]
fn sort_filter_duplicates_and_columns_run_from_their_dialogs() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    open(&mut cx, &book);
    run(&mut cx, &book, "cell.goto", &["B3"]);
    update(&mut cx, &book, |book, cx| {
        book.ask_data(crate::data::Asked::Sort, cx)
    });
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Sort { .. }));
    update(&mut cx, &book, |book, cx| {
        let Some(Dialog::Sort {
            picked,
            keys,
            header,
            ..
        }) = &mut book.dialog
        else {
            unreachable!("checked above");
        };
        assert_eq!(picked.top, ["Name", "Qty", "City"]);
        assert!(*header, "the kernel sees headers");
        assert_eq!(crate::data::title(picked, 1, *header), "Qty");
        *keys = vec![SortKey {
            column: "1".into(),
            rising: true,
        }];
        book.data_asked(cx);
    });
    assert_eq!(cells(&mut cx, &book, "A2:A5"), "b\nc\na\na");

    run(&mut cx, &book, "cell.goto", &["A1"]);
    update(&mut cx, &book, |book, cx| {
        book.ask_data(crate::data::Asked::Filter, cx)
    });
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Filter { .. }));
    update(&mut cx, &book, |book, cx| {
        if let Some(Dialog::Filter { rules, .. }) = &mut book.dialog {
            *rules = vec![FilterRule {
                column: "2".into(),
                test: Test::Equals,
                value: "x".into(),
            }];
        }
        book.data_asked(cx);
    });
    assert_eq!(dropped(&mut cx, &book, 2), ["y"]);
    run(&mut cx, &book, "data.unfilter", &[]);

    run(&mut cx, &book, "cell.goto", &["A2"]);
    update(&mut cx, &book, |book, cx| {
        book.ask_data(crate::data::Asked::Dedupe, cx)
    });
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Dedupe { .. }));
    update(&mut cx, &book, |book, cx| book.data_asked(cx));
    assert_eq!(cells(&mut cx, &book, "A1:A5"), "Name\nb\nc\na");

    run(&mut cx, &book, "cell.goto", &["A7"]);
    run(&mut cx, &book, "cell.enter", &["p;q;r"]);
    run(&mut cx, &book, "cell.goto", &["A7"]);
    update(&mut cx, &book, |book, cx| book.ask_columns(cx));
    asked(&cx, &book, |dialog| {
        matches!(dialog, Dialog::Columns { .. })
    });
    update(&mut cx, &book, |book, cx| {
        if let Some(Dialog::Columns { separators, .. }) = &mut book.dialog {
            *separators = vec!["59".into()];
        }
        book.columns_asked(cx);
    });
    assert_eq!(cells(&mut cx, &book, "A7:C7"), "p\tq\tr");
}

#[test]
fn a_column_button_lists_validation_offers_and_formats_list() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    open(&mut cx, &book);
    run(&mut cx, &book, "cell.goto", &["A1"]);
    run(&mut cx, &book, "data.autofilter", &[]);
    let id = cx.update(|app| book.read(app).core.active.expect("a tab"));
    update(&mut cx, &book, |book, cx| {
        book.ask_column(id, &Request::Filter { column: 0, row: 0 }, cx)
    });
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Column { .. }));
    update(&mut cx, &book, |book, cx| {
        let Some(Dialog::Column {
            entries, picked, ..
        }) = &mut book.dialog
        else {
            unreachable!("checked above");
        };
        assert_eq!(picked.len(), entries.len(), "nothing filtered yet");
        *picked = vec!["c".into()];
        book.column_asked(None, cx);
    });
    assert_eq!(dropped(&mut cx, &book, 0), ["a", "b"]);
    run(&mut cx, &book, "data.unfilter", &[]);

    run(&mut cx, &book, "cell.goto", &["E2"]);
    update(&mut cx, &book, |book, cx| book.ask_rule(cx));
    asked(&cx, &book, |dialog| {
        matches!(dialog, Dialog::Validation { .. })
    });
    update(&mut cx, &book, |book, cx| {
        let Some(Dialog::Validation { rule, fields, .. }) = &mut book.dialog else {
            unreachable!("checked above");
        };
        rule.kind = "list".into();
        rule.show_input = true;
        fields
            .list
            .update(cx, |list, cx| list.set_text("Red\nGreen", cx));
        fields
            .input_message
            .update(cx, |note, cx| note.set_text("Pick a color", cx));
        book.rule_asked(cx);
    });
    run(&mut cx, &book, "cell.goto", &["E3"]);
    run(&mut cx, &book, "cell.goto", &["E2"]);
    settle(&cx, |app| {
        book.read(app)
            .active_tab()
            .and_then(|tab| tab.extra.rule.as_ref())
            .is_some_and(|(at, rule)| at.1 == "E2" && rule.input_message == "Pick a color")
    });
    update(&mut cx, &book, |book, cx| {
        book.offer_choices(id, (4, 1), cx)
    });
    settle(&cx, |app| book.read(app).choices.is_some());
    let values = cx.update(|app| {
        let choices = book.read(app).choices.as_ref().expect("the menu");
        choices.values.clone()
    });
    assert_eq!(values, ["Red", "Green"]);

    run(&mut cx, &book, "cell.goto", &["B2:B5"]);
    cx.update_window(window.into(), |_, window, app| {
        book.update(app, |book, cx| book.ask_conditional(window, cx))
    })
    .expect("the window lives");
    update(&mut cx, &book, |book, cx| {
        if let Some(Dialog::Conditional { value1, .. }) = &book.dialog {
            value1.update(cx, |value, cx| value.set_text("2", cx));
        }
        book.conditional_asked(cx);
    });
    quiet(&cx, &book);
    update(&mut cx, &book, |book, cx| book.ask_formats(cx));
    asked(&cx, &book, |dialog| {
        matches!(dialog, Dialog::Formats { .. })
    });
    let style = cx.update(|app| match &book.read(app).dialog {
        Some(Dialog::Formats { formats, .. }) => formats[0].style.clone(),
        _ => unreachable!("checked above"),
    });
    assert_eq!(style, "Good");
    update(&mut cx, &book, |book, cx| book.format_removed(cx));
    quiet(&cx, &book);
    update(&mut cx, &book, |book, cx| book.ask_formats(cx));
    assert!(
        cx.update(|app| book.read(app).dialog.is_none()),
        "none left"
    );
}

#[test]
fn the_panel_reads_number_formats_and_find_counts_cells() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    open(&mut cx, &book);
    run(&mut cx, &book, "cell.goto", &["B2"]);
    run(&mut cx, &book, "number.percent", &[]);
    update(&mut cx, &book, |book, cx| {
        let tab = book.active_tab().expect("a tab");
        let shown = (tab.view.clone(), tab.doc.clone());
        book.panel
            .update(cx, |panel, cx| panel.show(Some((&shown.0, &shown.1)), cx));
    });
    let kind = |app: &gpui::App| book.read(app).panel.read(app).number.clone();
    settle(&cx, |app| {
        kind(app).is_some_and(|(_, kind)| kind == "percent")
    });
    let panel = cx.update(|app| book.read(app).panel.clone());
    cx.update(|app| panel.update(app, |panel, cx| panel.apply_code("[ZZZ]0", cx)));
    quiet(&cx, &book);
    cx.update(|app| panel.update(app, |panel, cx| panel.apply_code("0.0", cx)));
    settle(&cx, |app| kind(app).is_some_and(|(code, _)| code == "0.0"));

    cx.update_window(window.into(), |_, window, app| {
        book.update(app, |book, cx| {
            book.on_find(&ely_office::FindReplace, window, cx)
        })
    })
    .expect("the window lives");
    let query = cx.update(|app| {
        let finder = book.read(app).core.finder.as_ref().expect("the finder");
        finder.query.clone()
    });
    cx.update(|app| query.update(app, |query, cx| query.set_text("a", cx)));
    settle(&cx, |app| {
        book.read(app)
            .core
            .finder
            .as_ref()
            .is_some_and(|finder| finder.all.len() == 3)
    });
}

/// Two blocks: keys in A, other numbers in C.
const BLOCKS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="S">
 <table:table-row><table:table-cell><text:p>K</text:p></table:table-cell><table:table-cell/><table:table-cell><text:p>L</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell office:value-type="float" office:value="3"><text:p>3</text:p></table:table-cell><table:table-cell/><table:table-cell office:value-type="float" office:value="9"><text:p>9</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell office:value-type="float" office:value="1"><text:p>1</text:p></table:table-cell><table:table-cell/><table:table-cell office:value-type="float" office:value="7"><text:p>7</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell office:value-type="float" office:value="2"><text:p>2</text:p></table:table-cell><table:table-cell/><table:table-cell office:value-type="float" office:value="8"><text:p>8</text:p></table:table-cell></table:table-row>
 </table:table></office:spreadsheet></office:body>
</office:document>"#;

#[test]
fn a_column_button_sorts_its_own_block_not_the_cursors() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    update(&mut cx, &book, |book, cx| {
        book.new_from_text("blocks.fods", BLOCKS, cx)
    });
    settle(&cx, |app| sheet_names(app, &book) == ["S"]);
    run(&mut cx, &book, "cell.goto", &["A1"]);
    run(&mut cx, &book, "data.autofilter", &[]);
    run(&mut cx, &book, "cell.goto", &["C3"]);
    let id = cx.update(|app| book.read(app).core.active.expect("a tab"));
    update(&mut cx, &book, |book, cx| {
        book.ask_column(id, &Request::Filter { column: 0, row: 0 }, cx)
    });
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Column { .. }));
    update(&mut cx, &book, |book, cx| book.column_asked(Some(true), cx));
    assert_eq!(cells(&mut cx, &book, "A2:A4"), "1\n2\n3");
    assert_eq!(cells(&mut cx, &book, "C2:C4"), "9\n7\n8");
}

#[test]
fn the_column_search_redraws_the_list_and_keeps_hidden_ticks() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    open(&mut cx, &book);
    run(&mut cx, &book, "cell.goto", &["A1"]);
    run(&mut cx, &book, "data.autofilter", &[]);
    let id = cx.update(|app| book.read(app).core.active.expect("a tab"));
    update(&mut cx, &book, |book, cx| {
        book.ask_column(id, &Request::Filter { column: 0, row: 0 }, cx)
    });
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Column { .. }));
    let redraws = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = redraws.clone();
    cx.update(|app| app.observe(&book, move |_, _| counted.set(counted.get() + 1)).detach());
    let search = cx.update(|app| match &book.read(app).dialog {
        Some(Dialog::Column { search, .. }) => search.clone(),
        _ => unreachable!("checked above"),
    });
    quiet(&cx, &book);
    redraws.set(0);
    // Counted before the engine runs: only the search redraws.
    cx.update(|app| search.update(app, |field, cx| field.set_text("C", cx)));
    assert!(redraws.get() > 0, "a search redraws the list");
    // Untouched a and b stay ticked behind the search.
    update(&mut cx, &book, |book, cx| book.column_asked(None, cx));
    assert!(dropped(&mut cx, &book, 0).is_empty(), "nothing dropped");
}
