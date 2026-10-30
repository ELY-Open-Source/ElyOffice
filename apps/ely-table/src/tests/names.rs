use super::*;

/// The active workbook's names, from the kernel.
fn names(app: &gpui::App, book: &Entity<Book>) -> Vec<ely_lok::sheet::Name> {
    let book = book.read(app);
    let tab = book.active_tab().expect("a tab");
    let mut worker = book.core.worker.lock().expect("the worker lock");
    let json = worker
        .values(&tab.doc, ely_lok::sheet::NAMES)
        .expect("names");
    ely_lok::sheet::names(&json).expect("the names parse")
}

/// Opens the define dialog in the test window.
fn ask_define(cx: &mut TestAppContext, book: &Entity<Book>) {
    cx.update(|app| book.update(app, |book, cx| book.ask_define(cx)));
}

/// The define dialog's range text, once it is open.
fn asked_range(cx: &TestAppContext, book: &Entity<Book>) -> String {
    settle(cx, |app| {
        matches!(book.read(app).dialog, Some(Dialog::Define { .. }))
    });
    cx.update(|app| match &book.read(app).dialog {
        Some(Dialog::Define { range, .. }) => range.read(app).text().to_string(),
        _ => unreachable!("checked above"),
    })
}

/// Waits until every queued input of every tab landed.
fn quiet(cx: &TestAppContext, book: &Entity<Book>) {
    settle(cx, |app| {
        book.read(app)
            .core
            .tabs
            .iter()
            .all(|tab| tab.view.read(app).settled())
    });
}

#[test]
fn a_name_is_checked_then_defined_on_the_selection() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &book);
    let sheet = cx.update(|app| sheet_names(app, &book)[0].clone());
    update(&mut cx, &book, |book, cx| {
        book.run_active("cell.goto", &["B2:C3"], cx)
    });
    settle(&cx, |app| {
        book.read(app)
            .active_tab()
            .is_some_and(|tab| tab.view.read(app).cell().address == "B2:C3")
    });
    update(&mut cx, &book, |book, cx| book.ask_define(cx));
    settle(&cx, |app| {
        matches!(book.read(app).dialog, Some(Dialog::Define { .. }))
    });
    let define = |cx: &mut TestAppContext, name: &str| {
        cx.update(|app| {
            book.update(app, |book, cx| {
                if let Some(Dialog::Define { name: field, .. }) = &book.dialog {
                    let name = name.to_string();
                    field.update(cx, |field, cx| field.set_text(name, cx));
                }
                book.define(cx);
            })
        });
    };
    // Checks answer in order: the cell name first.
    define(&mut cx, "C4");
    define(&mut cx, "Area");
    settle(&cx, |app| book.read(app).dialog.is_none());
    settle(&cx, |app| {
        book.read(app)
            .active_tab()
            .is_some_and(|tab| tab.view.read(app).settled())
    });
    let defined = cx.update(|app| names(app, &book));
    assert_eq!(defined.len(), 1);
    assert_eq!(
        (defined[0].name.as_str(), defined[0].formula.as_str()),
        ("Area", format!("{sheet}!$B$2:$C$3").as_str()),
        "the selection, sheet included"
    );
}

#[test]
fn define_waits_for_queued_input_and_reads_named_selections() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &book);
    let sheet = cx.update(|app| sheet_names(app, &book)[0].clone());
    update(&mut cx, &book, |book, cx| {
        book.run_active("cell.goto", &["B2:C3"], cx)
    });
    ask_define(&mut cx, &book);
    assert_eq!(
        asked_range(&cx, &book),
        format!("{sheet}!$B$2:$C$3"),
        "the selection after the queued move"
    );
    update(&mut cx, &book, |book, cx| {
        book.dialog = None;
        let range = format!("'{sheet}'!$D$4:$E$5");
        book.run_active("names.define", &["Area", &range, ""], cx);
        book.run_active("cell.goto", &["A1"], cx);
        book.run_active("cell.goto", &["Area"], cx);
    });
    quiet(&cx, &book);
    let range = cx.update(|app| {
        let names = names(app, &book);
        names[0].range.clone()
    });
    assert!(range.ends_with("!$D$4:$E$5"), "{range}");
    ask_define(&mut cx, &book);
    assert_eq!(
        asked_range(&cx, &book),
        range,
        "a named selection keeps its range"
    );
}

#[test]
fn the_names_list_stays_with_its_workbook_and_scope() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    for _ in 0..2 {
        update(&mut cx, &book, |book, cx| book.new_blank(cx));
    }
    settle(&cx, |app| {
        let tabs = &book.read(app).core.tabs;
        tabs.len() == 2 && tabs.iter().all(|tab| !tab.extra.sheets.list.is_empty())
    });
    let first = cx.update(|app| book.read(app).core.tabs[0].id);
    let sheet = cx.update(|app| {
        book.read(app).core.tabs[0].extra.sheets.list[0]
            .name
            .clone()
    });
    update(&mut cx, &book, |book, cx| {
        book.core.active = Some(first);
        book.run_active("sheet.insert", &["Other", "2"], cx);
        book.run_active(
            "names.define",
            &["Spot", &format!("'{sheet}'!$A$5"), ""],
            cx,
        );
        book.run_active("names.define", &["Spot", "'Other'!$C$7", "Other"], cx);
        book.run_active("names.define", &["Local", "'Other'!$B$2", "Other"], cx);
        book.show_sheet(0, cx);
    });
    quiet(&cx, &book);
    cx.update(|app| {
        book.update(app, |book, cx| {
            book.run_active("cell.enter", &["7"], cx);
            book.names_in(first, cx);
        })
    });
    settle(&cx, |app| {
        matches!(book.read(app).dialog, Some(Dialog::Names { .. }))
    });
    update(&mut cx, &book, |book, _| {
        book.core.active = Some(book.core.tabs[1].id)
    });
    let pick = |cx: &mut TestAppContext, name: &str, scope: &str| {
        update(cx, &book, |book, cx| {
            let Some(Dialog::Names { tab, names, .. }) = &book.dialog else {
                panic!("the names list is open");
            };
            assert_eq!(*tab, first, "the list stays with the asking workbook");
            let at = names
                .iter()
                .position(|known| known.name == name && known.scope == scope)
                .expect("the name is listed");
            book.choose_name(at, cx);
            book.act_on_name(crate::formulas::NameAct::Go, cx);
        });
    };
    let shown = |cx: &mut TestAppContext| {
        quiet(cx, &book);
        let active = cx.update(|app| book.read(app).core.active);
        assert_eq!(active, Some(first), "the jump shows its workbook");
        update(cx, &book, |book, cx| book.refill(cx));
        let named = cx.update(|app| book.read(app).bars.name.read(app).text().to_string());
        let address = cx.update(|app| {
            book.read(app).core.tabs[0]
                .view
                .read(app)
                .cell()
                .address
                .clone()
        });
        assert_eq!(named, address, "the name box shows the target");
        cx.update(|app| {
            let tab = &book.read(app).core.tabs[0];
            (
                tab.extra.sheets.current,
                tab.view.read(app).cell().address.clone(),
            )
        })
    };
    // The sheet and the cursor report apart.
    let landed = |app: &gpui::App, part: usize, cells: &[&str], name: &str| {
        let tab = &book.read(app).core.tabs[0];
        let address = tab.view.read(app).cell().address.clone();
        tab.extra.sheets.current == part && (cells.contains(&address.as_str()) || address.starts_with(name))
    };
    pick(&mut cx, "Local", "Other");
    settle(&cx, |app| landed(app, 1, &["B2"], "Local"));
    let (part, address) = shown(&mut cx);
    assert!(
        part == 1 && (address == "B2" || address.starts_with("Local")),
        "{part} {address}"
    );
    cx.update(|app| {
        book.update(app, |book, cx| {
            book.names_in(first, cx);
        })
    });
    settle(&cx, |app| {
        matches!(book.read(app).dialog, Some(Dialog::Names { .. }))
    });
    update(&mut cx, &book, |book, _| {
        book.core.active = Some(book.core.tabs[1].id)
    });
    pick(&mut cx, "Spot", "");
    settle(&cx, |app| landed(app, 0, &["A5", "Spot"], "Spot"));
    let (part, address) = shown(&mut cx);
    assert!(
        part == 0 && (address == "A5" || address == "Spot"),
        "{part} {address}"
    );
}

#[test]
fn a_waiting_definition_keeps_its_workbook_sheets() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    for _ in 0..2 {
        update(&mut cx, &book, |book, cx| book.new_blank(cx));
    }
    settle(&cx, |app| {
        let tabs = &book.read(app).core.tabs;
        tabs.len() == 2 && tabs.iter().all(|tab| !tab.extra.sheets.list.is_empty())
    });
    let (first, second) = cx.update(|app| {
        let tabs = &book.read(app).core.tabs;
        (tabs[0].id, tabs[1].id)
    });
    for (id, name) in [(first, "Origin"), (second, "Destination")] {
        update(&mut cx, &book, |book, cx| {
            book.core.active = Some(id);
            book.run_active("sheet.rename", &[name, "1"], cx);
        });
    }
    settle(&cx, |app| {
        book.read(app)
            .core
            .tabs
            .iter()
            .zip(["Origin", "Destination"])
            .all(|(tab, name)| {
                tab.extra
                    .sheets
                    .list
                    .first()
                    .is_some_and(|sheet| sheet.name == name)
            })
    });
    cx.update(|app| {
        book.update(app, |book, cx| {
            book.core.active = Some(first);
            book.run_active("cell.goto", &["B2:C3"], cx);
            book.ask_define(cx);
        })
    });
    settle(&cx, |app| {
        matches!(book.read(app).dialog, Some(Dialog::Define { .. }))
    });
    update(&mut cx, &book, |book, _| book.core.active = Some(second));
    let scopes = cx.update(|app| book.read(app).define_scopes());
    assert_eq!(
        scopes,
        Some(vec!["Origin".to_string()]),
        "the asking workbook's sheets"
    );
    update(&mut cx, &book, |book, cx| {
        if let Some(Dialog::Define { name, scope, .. }) = &mut book.dialog {
            *scope = "Origin".into();
            name.update(cx, |name, cx| name.set_text("Area", cx));
        }
        book.define(cx);
        book.core.active = Some(first);
    });
    settle(&cx, |app| book.read(app).dialog.is_none());
    quiet(&cx, &book);
    let defined = cx.update(|app| names(app, &book));
    assert!(
        defined
            .iter()
            .any(|name| name.name == "Area" && name.scope == "Origin"),
        "{defined:?}"
    );
}

#[test]
fn a_rename_stays_with_its_workbook() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    for _ in 0..2 {
        update(&mut cx, &book, |book, cx| book.new_blank(cx));
    }
    settle(&cx, |app| {
        let tabs = &book.read(app).core.tabs;
        tabs.len() == 2 && tabs.iter().all(|tab| !tab.extra.sheets.list.is_empty())
    });
    let (first, second) = cx.update(|app| {
        let tabs = &book.read(app).core.tabs;
        (tabs[0].id, tabs[1].id)
    });
    let before = cx.update(|app| {
        book.read(app).core.tabs[1].extra.sheets.list[0]
            .name
            .clone()
    });
    cx.update_window(window.into(), |_, window, app| {
        book.update(app, |book, cx| {
            book.core.active = Some(first);
            book.ask_rename(window, cx);
            book.core.active = Some(second);
            book.rename_asked("Kept", cx);
        })
    })
    .expect("the window lives");
    settle(&cx, |app| {
        book.read(app).core.tabs[0].extra.sheets.list[0].name == "Kept"
    });
    assert_eq!(
        cx.update(|app| book.read(app).core.tabs[1].extra.sheets.list[0]
            .name
            .clone()),
        before,
        "the other workbook keeps its sheet name"
    );
}
