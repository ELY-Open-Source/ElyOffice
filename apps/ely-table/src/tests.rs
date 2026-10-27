use std::path::Path;

use ely_office::prelude::*;
use gpui::{AppContext as _, Entity, TestAppContext, WindowHandle};

use crate::book::Book;
use crate::dialog::Dialog;

struct Blank;

impl gpui::Render for Blank {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::Empty
    }
}

/// An unrendered Book over a fresh worker, if LOK exists.
fn book(cx: &mut TestAppContext, dir: &Path) -> Option<(WindowHandle<Blank>, Entity<Book>)> {
    let install = ely_lok::install_dir().ok()?;
    let worker = ely_office::lok::test_worker(&install, &dir.join("profile"));
    let work = dir.join("work");
    std::fs::create_dir_all(&work).expect("the work dir");
    cx.update(|cx| {
        ely_ui::settings::install_defaults(cx);
        ely_ui::i18n::install(cx);
    });
    let window = cx.add_window(|_, _| Blank);
    let book = cx
        .update_window(window.into(), |_, window, app| {
            let book = app.new(|cx| Book::new(worker, work, window, cx));
            book.update(app, |book, _| {
                book.core.window = Some(window.window_handle())
            });
            book
        })
        .expect("the window lives");
    Some((window, book))
}

/// Steps the scheduler until `done` or ten seconds pass.
#[track_caller]
fn settle(cx: &TestAppContext, done: impl Fn(&gpui::App) -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if !cx.executor().tick() {
            cx.executor()
                .advance_clock(std::time::Duration::from_millis(20));
        }
        if cx.update(|app| done(app)) {
            return;
        }
    }
    panic!("the state never settled");
}

/// Waits until every tab's input landed and was read.
#[track_caller]
fn fresh(cx: &TestAppContext, book: &Entity<Book>) {
    settle(cx, |app| {
        let book = book.read(app);
        let mut worker = book.core.worker.lock().expect("the worker lock");
        book.core.tabs.iter().all(|tab| {
            tab.view.read(app).settled()
                && tab.extra.read_at.is_some() && tab.extra.read_at.as_ref() == worker.now(&tab.doc).ok().as_ref()
        })
    });
}

fn update(
    cx: &mut TestAppContext,
    book: &Entity<Book>,
    f: impl FnOnce(&mut Book, &mut gpui::Context<Book>),
) {
    cx.update(|app| book.update(app, f));
}

fn sheet_names(app: &gpui::App, book: &Entity<Book>) -> Vec<String> {
    book.read(app).active_tab().map_or_else(Vec::new, |tab| {
        tab.extra
            .sheets
            .list
            .iter()
            .map(|sheet| sheet.name.clone())
            .collect()
    })
}

/// A blank workbook, loaded and read.
fn blank(cx: &mut TestAppContext, book: &Entity<Book>) {
    update(cx, book, |book, cx| book.new_blank(cx));
    settle(cx, |app| sheet_names(app, book).len() == 1);
}

#[test]
fn a_new_workbook_saves_xlsx_and_keeps_its_sheets() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &book);
    update(&mut cx, &book, |book, cx| book.run_active("cell.enter", &["42"], cx));
    fresh(&cx, &book);
    update(&mut cx, &book, |book, cx| book.run_active("sheet.insert", &["Plan", "2"], cx));
    let first = cx.update(|app| sheet_names(app, &book)[0].clone());
    settle(&cx, |app| {
        sheet_names(app, &book) == [first.as_str(), "Plan"]
    });
    let path = dir.path().join("book.xlsx");
    let id = cx.update(|app| book.read(app).core.tabs[0].id);
    cx.update_window(window.into(), |_, window, app| {
        book.update(app, |book, cx| {
            book.write_as(id, path.clone(), None, window, cx)
        })
    })
    .expect("the window lives");
    settle(&cx, |app| book.read(app).core.tabs[0].name == "book.xlsx");
    update(&mut cx, &book, |book, cx| {
        book.close_tab(id, cx);
        book.open_file(path.clone(), cx);
    });
    settle(&cx, |app| {
        sheet_names(app, &book) == [first.as_str(), "Plan"]
    });
    let bytes = std::fs::read(&path).expect("the file");
    assert!(bytes.starts_with(b"PK"), "an XLSX zip");
}

#[test]
fn deleting_names_what_refers_and_merging_asks_when_data_would_hide() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &book);
    let first = cx.update(|app| sheet_names(app, &book)[0].clone());
    let formula = format!("='{first}'!A1*2");
    update(&mut cx, &book, |book, cx| {
        book.run_active("cell.enter", &["1"], cx);
        book.run_active("cell.goto", &["B1"], cx);
        book.run_active("cell.enter", &["b"], cx);
    });
    fresh(&cx, &book);
    update(&mut cx, &book, |book, cx| {
        book.run_active("sheet.insert", &["Uses", "2"], cx);
        book.run_active("cell.enter", &[&formula], cx);
    });
    settle(&cx, |app| sheet_names(app, &book).len() == 2);
    update(&mut cx, &book, |book, cx| {
        let tab = book.active_tab().expect("a tab");
        let mut worker = book.core.worker.lock().expect("the worker lock");
        tab.doc.show_sheet(&mut worker, 0).expect("Sheet1 shows");
        drop(worker);
        cx.notify();
    });
    settle(&cx, |app| {
        book.read(app)
            .active_tab()
            .is_some_and(|tab| tab.extra.sheets.current == 0)
    });
    update(&mut cx, &book, |book, cx| {
        book.ask_delete(book.core.active, cx)
    });
    settle(&cx, |app| {
        matches!(book.read(app).dialog, Some(Dialog::Delete { .. }))
    });
    let referred = cx.update(|app| match &book.read(app).dialog {
        Some(Dialog::Delete { references, .. }) => references.formulas,
        _ => 0,
    });
    assert_eq!(referred, 1, "the formula on Uses is named");
    update(&mut cx, &book, |book, cx| {
        book.dialog = None;
        book.run_active("cell.goto", &["A1:B1"], cx);
        book.merge(book.core.active, cx);
    });
    settle(&cx, |app| {
        matches!(book.read(app).dialog, Some(Dialog::Merge { .. }))
    });
    update(&mut cx, &book, |book, cx| {
        book.dialog = None;
        book.run_active("cell.goto", &["C1:D1"], cx);
        book.merge(book.core.active, cx);
    });
    settle(&cx, |app| {
        let book = book.read(app);
        book.dialog.is_none()
            && book.active_tab().is_some_and(|tab| {
                tab.view.read(app).settled() && tab.view.read(app).cell().address == "C1:D1"
            })
    });
}

#[test]
fn a_queued_entry_lands_before_the_sheet_switches() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &book);
    update(&mut cx, &book, |book, cx| {
        book.run_active("sheet.insert", &["Two", "2"], cx);
    });
    settle(&cx, |app| sheet_names(app, &book).len() == 2);
    update(&mut cx, &book, |book, cx| book.show_sheet(0, cx));
    settle(&cx, |app| {
        book.read(app)
            .active_tab()
            .is_some_and(|tab| tab.extra.sheets.current == 0 && tab.view.read(app).settled())
    });
    update(&mut cx, &book, |book, cx| {
        book.run_active("cell.enter", &["42"], cx);
        book.show_sheet(1, cx);
        book.run_active("cell.enter", &["99"], cx);
    });
    settle(&cx, |app| {
        book.read(app)
            .active_tab()
            .is_some_and(|tab| tab.extra.sheets.current == 1 && tab.view.read(app).settled())
    });
    let path = dir.path().join("two.fods");
    cx.update(|app| {
        let book = book.read(app);
        let mut worker = book.core.worker.lock().expect("the worker lock");
        let doc = &book.core.tabs[0].doc;
        doc.save_as(&mut worker, &ely_lok::file_url(&path), "fods")
            .expect("the save");
    });
    let xml = std::fs::read_to_string(&path).expect("the save");
    let split = xml.find("table:name=\"Two\"").expect("the second sheet");
    let (first, second) = xml.split_at(split);
    assert!(
        first.contains("office:value=\"42\""),
        "42 stays on the first sheet"
    );
    assert!(
        second.contains("office:value=\"99\""),
        "99 goes to the second"
    );
}

#[test]
fn a_switch_stays_with_its_workbook() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let current = |app: &gpui::App, at: usize| book.read(app).core.tabs[at].extra.sheets.current;
    for _ in 0..2 {
        update(&mut cx, &book, |book, cx| book.new_blank(cx));
    }
    settle(&cx, |app| {
        let tabs = &book.read(app).core.tabs;
        tabs.len() == 2 && tabs.iter().all(|tab| tab.extra.sheets.list.len() == 1)
    });
    for at in 0..2 {
        update(&mut cx, &book, |book, cx| {
            book.core.active = Some(book.core.tabs[at].id);
            book.run_active("sheet.insert", &["Two", "2"], cx);
            book.show_sheet(0, cx);
        });
    }
    settle(&cx, |app| {
        book.read(app)
            .core
            .tabs
            .iter()
            .all(|tab| tab.extra.sheets.list.len() == 2 && tab.view.read(app).settled())
    });
    update(&mut cx, &book, |book, cx| {
        book.core.active = Some(book.core.tabs[0].id);
        book.run_active("cell.enter", &["42"], cx);
        book.show_sheet(1, cx);
        book.core.active = Some(book.core.tabs[1].id);
    });
    settle(&cx, |app| {
        book.read(app)
            .core
            .tabs
            .iter()
            .all(|tab| tab.view.read(app).settled())
    });
    settle(&cx, |app| current(app, 0) == 1);
    assert_eq!(
        cx.update(|app| current(app, 1)),
        0,
        "the other workbook stays"
    );
}

#[test]
fn the_bars_follow_the_active_workbook() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    for _ in 0..2 {
        update(&mut cx, &book, |book, cx| book.new_blank(cx));
    }
    settle(&cx, |app| book.read(app).core.tabs.len() == 2);
    update(&mut cx, &book, |book, cx| {
        book.core.active = Some(book.core.tabs[1].id);
        book.run_active("cell.goto", &["C3"], cx);
    });
    let address = |app: &gpui::App, at: usize| {
        book.read(app).core.tabs[at]
            .view
            .read(app)
            .cell()
            .address
            .clone()
    };
    settle(&cx, |app| {
        address(app, 0) == "A1" && address(app, 1) == "C3"
    });
    let name = |cx: &mut TestAppContext, at: usize| {
        update(cx, &book, |book, cx| {
            book.core.active = Some(book.core.tabs[at].id);
            book.refill(cx);
        });
        cx.update(|app| book.read(app).bars.name.read(app).text().to_string())
    };
    assert_eq!(name(&mut cx, 0), "A1", "the first workbook's cell");
    assert_eq!(name(&mut cx, 1), "C3", "then the second's");
    assert_eq!(name(&mut cx, 0), "A1", "and back");
}

#[test]
fn a_delete_stays_with_its_workbook() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let count = |app: &gpui::App, at: usize| book.read(app).core.tabs[at].extra.sheets.list.len();
    for _ in 0..2 {
        update(&mut cx, &book, |book, cx| book.new_blank(cx));
    }
    settle(&cx, |app| book.read(app).core.tabs.len() == 2);
    for (at, name) in [(0, "OriginTwo"), (1, "OtherTwo")] {
        fresh(&cx, &book);
        update(&mut cx, &book, |book, cx| {
            book.core.active = Some(book.core.tabs[at].id);
            book.run_active("sheet.insert", &[name, "2"], cx);
            book.show_sheet(1, cx);
        });
    }
    settle(&cx, |app| count(app, 0) == 2 && count(app, 1) == 2);
    update(&mut cx, &book, |book, cx| {
        book.core.active = Some(book.core.tabs[0].id);
        book.run_active("cell.enter", &["42"], cx);
        book.ask_delete(book.core.active, cx);
    });
    settle(&cx, |app| {
        matches!(book.read(app).dialog, Some(Dialog::Delete { .. }))
    });
    // The user moves on while the dialog is open.
    update(&mut cx, &book, |book, _| {
        book.core.active = Some(book.core.tabs[1].id)
    });
    let asked = cx.update(|app| match &book.read(app).dialog {
        Some(Dialog::Delete { name, .. }) => name.clone(),
        _ => String::new(),
    });
    assert_eq!(
        asked, "OriginTwo",
        "the dialog names the first workbook's sheet"
    );
    update(&mut cx, &book, |book, cx| book.delete_asked(cx));
    settle(&cx, |app| count(app, 0) == 1);
    assert_eq!(
        cx.update(|app| count(app, 1)),
        2,
        "the other workbook keeps both"
    );
}

mod analysis;
mod chinese;
mod aids;
mod data;
mod edges;
mod menu;
mod merge;
mod names;
mod numbers;
mod output;
mod rules;
mod undo;
