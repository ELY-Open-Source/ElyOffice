use ely_lok::sheet::Validation;

use super::data::{open, quiet, run};
use super::*;

/// The active workbook's cached rule for its current cell.
fn rule(app: &gpui::App, book: &Entity<Book>) -> Option<(String, Validation)> {
    let tab = book.read(app).active_tab()?;
    let (spot, rule) = tab.extra.rule.as_ref()?;
    Some((spot.1.clone(), rule.clone()))
}

#[test]
fn a_dialog_asked_from_a_workbook_left_behind_is_dropped() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    open(&mut cx, &book);
    update(&mut cx, &book, |book, cx| book.new_blank(cx));
    settle(&cx, |app| {
        let tabs = &book.read(app).core.tabs;
        tabs.len() == 2 && tabs.iter().all(|tab| !tab.extra.sheets.list.is_empty())
    });
    let (first, second) = cx.update(|app| {
        let tabs = &book.read(app).core.tabs;
        (tabs[0].id, tabs[1].id)
    });
    type Ask = fn(&mut Book, &mut gpui::Window, &mut gpui::Context<Book>);
    let asks: [Ask; 5] = [
        |book, _, cx| book.ask_rule(cx),
        |book, _, cx| book.ask_columns(cx),
        |book, _, cx| book.ask_formats(cx),
        |book, _, cx| book.ask(crate::analysis::Asked::Chart, cx),
        |book, _, cx| book.ask(crate::analysis::Asked::Pivot, cx),
    ];
    for ask in asks {
        update(&mut cx, &book, |book, cx| {
            book.core.set_active(Some(first));
            book.run_active("cell.goto", &["B2"], cx);
        });
        quiet(&cx, &book);
        let worker = cx.update(|app| book.read(app).core.worker.clone());
        // The read lands only after the switch.
        let held = worker.lock().expect("the worker lock");
        cx.update_window(window.into(), |_, window, app| {
            book.update(app, |book, cx| {
                book.dialog = None;
                ask(book, window, cx);
                book.core.set_active(Some(second));
            })
        })
        .expect("the window lives");
        drop(held);
        for _ in 0..200 {
            cx.executor().tick();
        }
        quiet(&cx, &book);
        assert!(
            cx.update(|app| book.read(app).dialog.is_none()),
            "no dialog for a workbook left behind"
        );
    }
}

#[test]
fn rules_set_in_place_reach_the_cell_and_clear() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    open(&mut cx, &book);
    run(&mut cx, &book, "cell.goto", &["E2"]);
    settle(&cx, |app| {
        rule(app, &book).is_some_and(|(at, rule)| at == "E2" && rule.kind == "any")
    });
    let list = Validation {
        kind: "list".into(),
        operator: "equal".into(),
        list: vec!["Red".into()],
        dropdown: true,
        show_input: true,
        input_message: "A color".into(),
        error_style: "stop".into(),
        ..Validation::default()
    };
    run(&mut cx, &book, "rules.validation", &[&list.to_json()]);
    settle(&cx, |app| {
        rule(app, &book).is_some_and(|(_, rule)| rule.kind == "list")
    });
    update(&mut cx, &book, |book, cx| book.clear_rule(cx));
    quiet(&cx, &book);
    settle(&cx, |app| {
        rule(app, &book).is_some_and(|(_, rule)| rule.kind == "any" && !rule.show_input)
    });
}

#[test]
fn the_panel_follows_a_sheet_switch_at_one_address() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    open(&mut cx, &book);
    run(&mut cx, &book, "sheet.insert", &["T", "2"]);
    settle(&cx, |app| sheet_names(app, &book) == ["S", "T"]);
    update(&mut cx, &book, |book, cx| {
        let tab = book.active_tab().expect("a tab");
        let (view, doc) = (tab.view.clone(), tab.doc.clone());
        book.panel
            .update(cx, |panel, cx| panel.show(Some((&view, &doc)), cx));
    });
    let kind = |app: &gpui::App| {
        let number = book.read(app).panel.read(app).number.clone();
        number.map(|(_, kind)| kind)
    };
    for (sheet, command, want) in [
        (0, "number.percent", "percent"),
        (1, "number.currency", "currency"),
    ] {
        update(&mut cx, &book, |book, cx| book.show_sheet(sheet, cx));
        run(&mut cx, &book, "cell.goto", &["A1"]);
        run(&mut cx, &book, command, &[]);
        settle(&cx, |app| kind(app).as_deref() == Some(want));
    }
    update(&mut cx, &book, |book, cx| book.show_sheet(0, cx));
    run(&mut cx, &book, "cell.goto", &["A1"]);
    settle(&cx, |app| kind(app).as_deref() == Some("percent"));
}

#[test]
fn a_format_code_lands_in_the_workbook_that_asked() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    open(&mut cx, &book);
    update(&mut cx, &book, |book, cx| book.new_blank(cx));
    settle(&cx, |app| {
        let tabs = &book.read(app).core.tabs;
        tabs.len() == 2 && tabs.iter().all(|tab| !tab.extra.sheets.list.is_empty())
    });
    let (first, second) = cx.update(|app| {
        let tabs = &book.read(app).core.tabs;
        (tabs[0].id, tabs[1].id)
    });
    let code = |cx: &mut TestAppContext, id: u64| {
        cx.update(|app| {
            let book = book.read(app);
            let tab = book.tab(id).expect("a tab");
            let mut worker = book.core.worker.lock().expect("the worker lock");
            worker.run(&tab.doc, "cell.goto", &["A1"]).expect("A1");
            let json = worker
                .values(&tab.doc, ely_lok::sheet::NUMBER)
                .expect("the format");
            ely_lok::sheet::number_format(&json).expect("parses").0
        })
    };
    let (panel, view, doc) = cx.update(|app| {
        book.update(app, |book, _| book.core.set_active(Some(first)));
        let book = book.read(app);
        let tab = book.tab(first).expect("a tab");
        (book.panel.clone(), tab.view.clone(), tab.doc.clone())
    });
    cx.update(|app| {
        panel.update(app, |panel, cx| {
            panel.show(Some((&view, &doc)), cx);
            panel.apply_code("0.000", cx);
        })
    });
    // The check is still on its way.
    update(&mut cx, &book, |book, _| book.core.set_active(Some(second)));
    settle(&cx, |app| {
        book.read(app)
            .core
            .tabs
            .iter()
            .all(|tab| tab.view.read(app).settled())
    });
    quiet(&cx, &book);
    for _ in 0..200 {
        cx.executor().tick();
    }
    assert_eq!(code(&mut cx, first), "0.000", "the asking workbook");
    assert_ne!(code(&mut cx, second), "0.000", "not the active one");
}
