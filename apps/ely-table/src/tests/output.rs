use ely_gpui_component::forms::TextInput;
use ely_lok::sheet::{self, Scale, csv::Column};

use super::data::{asked, cells, quiet, run};
use super::*;
use crate::output::Format;

/// The active tab's kernel read, after its input.
pub(super) fn read<T>(
    cx: &mut TestAppContext,
    book: &Entity<Book>,
    parse: fn(&str) -> std::io::Result<T>,
    command: &str,
) -> T {
    quiet(cx, book);
    cx.update(|app| {
        let book = book.read(app);
        let tab = book.active_tab().expect("a tab");
        let mut worker = book.core.worker.lock().expect("the worker lock");
        parse(&worker.values(&tab.doc, command).expect("the read")).expect("parses")
    })
}

pub(super) fn type_in(
    cx: &mut TestAppContext,
    book: &Entity<Book>,
    field: fn(&Dialog) -> &Entity<TextInput>,
    text: &str,
) {
    cx.update(|app| {
        let input = field(book.read(app).dialog.as_ref().expect("a dialog")).clone();
        input.update(app, |input, cx| input.set_text(text, cx));
    });
}

#[test]
fn a_csv_opens_through_its_choices_and_keeps_codes() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("accounts.txt");
    let mut text = "Account;Amount\n".to_string();
    text.push_str(&"123;2\n".repeat(18_000));
    text.push_str("007;1.5\n");
    std::fs::write(&path, text).expect("writes");
    update(&mut cx, &book, |book, cx| book.ask_import(path.clone(), cx));
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Import(pending) if !pending.reading));
    cx.update(|app| match book.read(app).dialog.as_ref() {
        Some(Dialog::Import(pending)) => {
            assert_eq!(pending.how.separator, ';', "the guessed separator");
            assert_eq!(pending.how.columns, [Column::Text, Column::Standard]);
        }
        _ => unreachable!("an import dialog"),
    });
    update(&mut cx, &book, |book, cx| book.import_asked(cx));
    settle(&cx, |app| {
        book.read(app).active_tab().is_some_and(|tab| tab.name == "accounts.txt")
    });
    assert_eq!(cells(&mut cx, &book, "A18002"), "007", "the zero stays");
    assert_eq!(cells(&mut cx, &book, "B18002"), "1.5");
    let saved = dir.path().join("accounts.xlsx");
    let id = cx.update(|app| book.read(app).core.tabs[0].id);
    cx.update_window(window.into(), |_, window, app| {
        book.update(app, |book, cx| book.write_as(id, saved.clone(), None, window, cx))
    })
    .expect("the window lives");
    settle(&cx, |app| book.read(app).core.tabs[0].name == "accounts.xlsx");
    update(&mut cx, &book, |book, cx| {
        book.close_tab(id, cx);
        book.open_file(saved.clone(), cx);
    });
    settle(&cx, |app| sheet_names(app, &book).len() == 1);
    assert_eq!(cells(&mut cx, &book, "A18002"), "007", "XLSX keeps it");
}

#[test]
fn a_wrong_encoding_blocks_the_import() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("latin.csv");
    std::fs::write(&path, b"Name\nCaf\xe9\n").expect("writes");
    update(&mut cx, &book, |book, cx| book.ask_import(path.clone(), cx));
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Import(pending) if !pending.reading));
    update(&mut cx, &book, |book, cx| {
        if let Some(Dialog::Import(pending)) = &mut book.dialog {
            pending.how.encoding = ely_lok::sheet::csv::Encoding::Utf8;
        }
        book.scan_import(cx);
    });
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Import(pending) if !pending.reading));
    update(&mut cx, &book, |book, cx| book.import_asked(cx));
    assert!(cx.update(|app| book.read(app).dialog.is_some()), "broken bytes stay out");
    assert!(cx.update(|app| book.read(app).core.loading.is_empty()));
}

#[test]
fn exports_write_the_shown_sheet_as_text() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &book);
    run(&mut cx, &book, "cell.enter", &["=1+2"]);
    run(&mut cx, &book, "cell.goto", &["B1"]);
    run(&mut cx, &book, "cell.enter", &["a;b"]);
    let id = cx.update(|app| book.read(app).core.tabs[0].id);
    // The kernel ends rows the system's way.
    let end = if cfg!(windows) { "\r\n" } else { "\n" };
    for (format, name, text) in [
        (Format::Csv, "out.csv", "3,a;b"),
        (Format::Tsv, "out.tsv", "3\ta;b"),
    ] {
        let path = dir.path().join(name);
        update(&mut cx, &book, |book, cx| book.export_to(id, path.clone(), format, cx));
        settle(&cx, |_| path.exists());
        quiet(&cx, &book);
        assert_eq!(std::fs::read_to_string(&path).expect("reads"), format!("{text}{end}"));
    }
    let pdf = dir.path().join("out.pdf");
    update(&mut cx, &book, |book, cx| book.export_to(id, pdf.clone(), Format::Pdf, cx));
    settle(&cx, |_| pdf.exists());
    quiet(&cx, &book);
    assert!(std::fs::read(&pdf).expect("reads").starts_with(b"%PDF"));
}

fn area(dialog: &Dialog) -> &Entity<TextInput> {
    match dialog {
        Dialog::Page { fields, .. } => &fields.area,
        _ => unreachable!("a page dialog"),
    }
}

fn rows(dialog: &Dialog) -> &Entity<TextInput> {
    match dialog {
        Dialog::Page { fields, .. } => &fields.rows,
        _ => unreachable!("a page dialog"),
    }
}

#[test]
fn page_setup_applies_titles_area_and_page() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &book);
    update(&mut cx, &book, |book, cx| book.ask_page(cx));
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Page { .. }));
    type_in(&mut cx, &book, area, "A1:");
    update(&mut cx, &book, |book, cx| book.page_asked(cx));
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Page { .. }));
    type_in(&mut cx, &book, area, "a1:c4");
    type_in(&mut cx, &book, rows, "1");
    update(&mut cx, &book, |book, cx| {
        if let Some(Dialog::Page {
            landscape, scale, ..
        }) = &mut book.dialog
        {
            (*landscape, *scale) = (true, Scale::Fit(1, 0));
        }
        book.page_asked(cx)
    });
    let layout = read(&mut cx, &book, sheet::print_layout, sheet::LAYOUT);
    assert_eq!(layout.areas, ["$A$1:$C$4"]);
    assert_eq!(layout.rows, Some((1, 1)));
    assert_eq!((layout.landscape, layout.scale), (true, Scale::Fit(1, 0)));
}

#[test]
fn a_new_link_names_its_unrefreshed_source() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let source = dir.path().join("source.ods");
    blank(&mut cx, &book);
    run(&mut cx, &book, "cell.enter", &["7"]);
    let name = cx.update(|app| sheet_names(app, &book)[0].clone());
    cx.update(|app| {
        let book = book.read(app);
        let tab = book.active_tab().expect("a tab");
        let mut worker = book.core.worker.lock().expect("the worker lock");
        tab.doc
            .save_as(&mut worker, &ely_lok::file_url(&source), "ods")
            .expect("the source");
    });
    settle(&cx, |app| book.read(app).active_tab().is_some_and(|tab| tab.extra.linked.is_some()));
    run(&mut cx, &book, "cell.goto", &["B1"]);
    let formula = format!("='{}/[source.ods]{name}'!A1", dir.path().display());
    run(&mut cx, &book, "cell.enter", &[&formula]);
    let url = ely_lok::file_url(&source);
    settle(&cx, |app| {
        book.read(app)
            .active_tab()
            .is_some_and(|tab| tab.extra.links == [url.clone()])
    });
}

#[test]
fn the_checked_bytes_import_even_if_the_file_changes() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("codes.csv");
    std::fs::write(&path, "Account\n00123\n").expect("writes");
    update(&mut cx, &book, |book, cx| book.ask_import(path.clone(), cx));
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Import(pending) if !pending.reading));
    std::fs::write(&path, b"Account\n999\nA\xffB\n").expect("rewrites");
    update(&mut cx, &book, |book, cx| book.import_asked(cx));
    settle(&cx, |app| book.read(app).active_tab().is_some());
    assert_eq!(cells(&mut cx, &book, "A2"), "00123", "what was checked");
}

#[test]
fn a_closed_dialogs_scan_never_lands_in_the_next() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let pending = |name: &str, text: &str| {
        Box::new(crate::text::Pending::new(dir.path().join(name), text.into(), false))
    };
    // Either order: one of them lands the old scan last.
    for old_first in [true, false] {
        let (old, next) = (pending("old.csv", "Old\n00123\n"), pending("next.csv", "Next,Amount\n1,2\n"));
        update(&mut cx, &book, |book, cx| {
            let (first, second) = match old_first {
                true => (Dialog::Import(old), Dialog::Import(next)),
                false => (Dialog::Import(next), Dialog::Import(old)),
            };
            book.dialog = Some(first);
            book.scan_import(cx);
            let first = book.dialog.replace(second);
            book.scan_import(cx);
            if !old_first {
                book.dialog = first;
            }
        });
        cx.run_until_parked();
        cx.update(|app| match book.read(app).dialog.as_ref() {
            Some(Dialog::Import(pending)) => {
                assert_eq!(pending.rows[0], ["Next", "Amount"], "old first {old_first}");
                assert_eq!(pending.how.columns, [Column::Standard, Column::Standard]);
            }
            _ => unreachable!("an import dialog"),
        });
    }
}

#[test]
fn the_preview_reads_what_the_kernel_imports() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("odd.csv");
    let text = "Id,Code\n \"00123\",1\n\ny,-12345678901234567890\n";
    std::fs::write(&path, text).expect("writes");
    update(&mut cx, &book, |book, cx| book.ask_import(path.clone(), cx));
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Import(pending) if !pending.reading));
    let preview = cx.update(|app| match book.read(app).dialog.as_ref() {
        Some(Dialog::Import(pending)) => pending.rows.clone(),
        _ => unreachable!("an import dialog"),
    });
    update(&mut cx, &book, |book, cx| book.import_asked(cx));
    settle(&cx, |app| book.read(app).active_tab().is_some());
    // Row 4 lines up only if the empty line counted.
    for (at, row) in preview.iter().enumerate().filter(|(_, row)| row.len() == 2) {
        for (column, value) in ["A", "B"].iter().zip(row) {
            let cell = format!("{column}{}", at + 1);
            assert_eq!(&cells(&mut cx, &book, &cell), value, "{cell} as previewed");
        }
    }
    assert_eq!(preview[3][1], "-12345678901234567890");
}

#[test]
fn text_files_wait_their_turn_for_the_import_dialog() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let paths = ["a.csv", "b.csv"].map(|name| dir.path().join(name));
    for (path, text) in paths.iter().zip(["first\n", "second\n"]) {
        std::fs::write(path, text).expect("writes");
    }
    // Another dialog is open; neither import may replace it.
    update(&mut cx, &book, |book, cx| {
        book.dialog = Some(Dialog::Merge { tab: 0 });
        for path in &paths {
            book.ask_import(path.clone(), cx);
        }
    });
    settle(&cx, |app| book.read(app).imports.len() == 2);
    cx.update(|app| assert!(matches!(book.read(app).dialog, Some(Dialog::Merge { .. }))));
    update(&mut cx, &book, |book, cx| {
        book.dialog = None;
        cx.notify();
    });
    for opened in 1..=2 {
        asked(&cx, &book, |dialog| matches!(dialog, Dialog::Import(pending) if !pending.reading));
        update(&mut cx, &book, |book, cx| book.import_asked(cx));
        settle(&cx, |app| book.read(app).core.tabs.len() == opened);
    }
    let mut names = cx.update(|app| book.read(app).core.tabs.iter().map(|tab| tab.name.clone()).collect::<Vec<_>>());
    names.sort();
    assert_eq!(names, ["a.csv", "b.csv"], "both files open");
}

#[test]
fn an_import_waits_behind_a_dialog_that_displaces_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &book);
    let path = dir.path().join("wait.csv");
    std::fs::write(&path, "Id,Ignored\n1E3,hidden\n").expect("writes");
    update(&mut cx, &book, |book, cx| book.ask_import(path.clone(), cx));
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Import(pending) if !pending.reading));
    update(&mut cx, &book, |book, _| {
        if let Some(Dialog::Import(pending)) = &mut book.dialog {
            pending.how.columns = vec![Column::Text, Column::Skip];
        }
    });
    window
        .update(&mut cx, |_, window, cx| {
            book.update(cx, |book, cx| book.on_export(&ely_ui::commands::Export, window, cx))
        })
        .expect("the window");
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Export { .. }));
    update(&mut cx, &book, |book, cx| {
        book.dialog = None;
        cx.notify();
    });
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Import(pending) if pending.path.ends_with("wait.csv")));
    cx.run_until_parked();
    update(&mut cx, &book, |book, cx| book.import_asked(cx));
    settle(&cx, |app| book.read(app).active_tab().is_some_and(|tab| tab.name == "wait.csv"));
    // Text keeps `1E3`; the skipped column stays empty.
    assert_eq!(cells(&mut cx, &book, "A2:B2").trim_end(), "1E3");
}

#[test]
fn closing_a_tab_takes_its_dialog_and_frees_the_import() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &book);
    window
        .update(&mut cx, |_, window, cx| {
            book.update(cx, |book, cx| book.on_export(&ely_ui::commands::Export, window, cx))
        })
        .expect("the window");
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Export { .. }));
    let id = cx.update(|app| book.read(app).core.tabs[0].id);
    update(&mut cx, &book, |book, cx| book.close_tab(id, cx));
    let path = dir.path().join("after.csv");
    std::fs::write(&path, "Name\nx\n").expect("writes");
    update(&mut cx, &book, |book, cx| book.ask_import(path.clone(), cx));
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Import(_)));
}

#[test]
fn a_whole_column_print_area_still_applies() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &book);
    update(&mut cx, &book, |book, cx| book.ask_page(cx));
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Page { .. }));
    type_in(&mut cx, &book, area, "A:D");
    update(&mut cx, &book, |book, cx| book.page_asked(cx));
    let layout = read(&mut cx, &book, sheet::print_layout, sheet::LAYOUT);
    assert_eq!(layout.areas.len(), 1, "set: {:?}", layout.areas);
    // The kernel's own text comes back into the dialog.
    update(&mut cx, &book, |book, cx| book.ask_page(cx));
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Page { .. }));
    let shown = cx.update(|app| match book.read(app).dialog.as_ref() {
        Some(dialog) => area(dialog).read(app).text().to_string(),
        None => unreachable!("a page dialog"),
    });
    assert_eq!(shown, layout.areas[0]);
    update(&mut cx, &book, |book, cx| {
        if let Some(Dialog::Page { landscape, .. }) = &mut book.dialog {
            *landscape = true;
        }
        book.page_asked(cx)
    });
    let layout = read(&mut cx, &book, sheet::print_layout, sheet::LAYOUT);
    assert!(layout.landscape, "applied with {shown}");
}

#[test]
fn the_default_encoding_follows_the_apps_language() {
    use ely_gpui_component::i18n::I18n;
    use ely_lok::sheet::csv::Encoding;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("gb.csv");
    // 你好 in GB18030: not UTF-8.
    std::fs::write(&path, [0xC4, 0xE3, 0xBA, 0xC3, b'\n']).expect("writes");
    for (tag, expected) in [("zh-CN", Encoding::Gb18030), ("en-US", Encoding::Windows1252)] {
        cx.update(|app| I18n::set_locale(tag, app));
        update(&mut cx, &book, |book, cx| book.ask_import(path.clone(), cx));
        asked(&cx, &book, |dialog| matches!(dialog, Dialog::Import(_)));
        let found = cx.update(|app| match book.read(app).dialog.as_ref() {
            Some(Dialog::Import(pending)) => pending.how.encoding,
            _ => unreachable!("an import dialog"),
        });
        assert_eq!(found, expected, "{tag}");
        update(&mut cx, &book, |book, cx| {
            book.dialog = None;
            cx.notify();
        });
    }
}
