//! A live workbook moves to a new window (W.1c).

use std::{cell::RefCell, rc::Rc};

use ely_office::{Core, Dialog, After, moving::Moving as _};
use gpui::AnyWindowHandle;

use super::*;

/// The first window, built as the app builds it.
fn first(cx: &mut TestAppContext, dir: &Path) -> Option<(Entity<Book>, AnyWindowHandle)> {
    // Real windows start recovery: never in the user's folder.
    ely_platform::test_root();
    let install = ely_lok::test_install()?;
    let (worker, stop) = ely_office::lok::test_worker(&install);
    let work = dir.join("work");
    std::fs::create_dir_all(&work).expect("the work dir");
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        ely_ui::settings::install_defaults(cx);
        ely_ui::install_theme(cx);
        ely_ui::i18n::install_as("en-US", cx);
        ely_ui::windows::allow_for_tests(ely_platform::AppId::Table, crate::window::more, cx);
    });
    let held = Rc::new(RefCell::new(None));
    let kept = Rc::clone(&held);
    let window = cx
        .update(|cx| {
            cx.open_window(Default::default(), move |window, cx| {
                let book = crate::window::book(move |cx| Core::new(worker, stop, work, cx), window, cx);
                kept.replace(Some(book.clone()));
                crate::window::root(&book, window, cx)
            })
        })
        .expect("a window");
    let window: AnyWindowHandle = window.into();
    cx.update(|cx| ely_ui::tray::testing::opened(ely_platform::AppId::Table, window, cx));
    let book = held.take().expect("the book");
    Some((book, window))
}

/// Draws the windows until `done`; plain ticks draw nothing.
#[track_caller]
fn wait(cx: &mut TestAppContext, done: impl Fn(&gpui::App) -> bool) {
    // The worker answers in real time, not test time.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while std::time::Instant::now() < deadline {
        cx.run_until_parked();
        if cx.update(|app| done(app)) {
            return;
        }
        cx.executor().advance_clock(std::time::Duration::from_millis(20));
    }
    panic!("the windows never showed it");
}

/// The book in the window in front.
fn front(cx: &mut TestAppContext) -> Entity<Book> {
    cx.update(|app| crate::window::front(None, app)).expect("a front book")
}

#[test]
fn an_edited_workbook_moves_and_keeps_its_undo() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((book, home)) = first(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    blank(&mut cx, &book);
    update(&mut cx, &book, |book, cx| book.new_blank(cx));
    wait(&mut cx, |app| book.read(app).core.tabs.len() == 2 && sheet_names(app, &book).len() == 1);
    update(&mut cx, &book, |book, cx| book.run_active("sheet.rename", &["Plan", "1"], cx));
    wait(&mut cx, |app| sheet_names(app, &book) == ["Plan"]);
    let id = cx.update(|app| book.read(app).core.active()).expect("a tab");
    let doc = cx.update(|app| book.read(app).tab(id).expect("the tab").doc.clone());
    cx.update(|app| ely_office::moving::to_new_window(book.clone(), id, app));
    wait(&mut cx, |app| app.windows().len() == 2);
    let other = front(&mut cx);
    assert_ne!(other, book, "a new window is in front");
    wait(&mut cx, |app| other.read(app).tab(id).is_some_and(|tab| tab.view.read(app).tile_count() > 0));
    let (left, moved) = cx.update(|app| (book.read(app).core.tabs.len(), other.read(app).core.tabs.len()));
    assert_eq!((left, moved), (1, 1), "one tab each, no blank added");
    cx.update(|app| {
        let tab = other.read(app).tab(id).expect("the moved tab");
        assert_eq!(tab.doc, doc, "the same kernel document, not reopened");
        assert!(tab.dirty(app), "its unsaved edit came along");
    });
    // The tab left behind has the keys at home.
    let kept = cx.update(|app| book.read(app).active_tab().map(|tab| tab.view.read(app).focus_handle().clone())).expect("a tab left");
    let keyed = home.update(&mut cx, |_, window, _| kept.is_focused(window)).expect("home is open");
    assert!(keyed, "typing at home reaches the tab left behind");
    // The kernel's own history works from the new window.
    update(&mut cx, &other, |other, cx| other.run_active("edit.undo", &[], cx));
    wait(&mut cx, |app| sheet_names(app, &other) != ["Plan"]);
    // The last tab leaves: its window closes.
    let last = cx.update(|app| book.read(app).core.active()).expect("the other tab");
    cx.update(|app| ely_office::moving::to_new_window(book.clone(), last, app));
    wait(&mut cx, |app| !app.windows().contains(&home));
    assert_eq!(cx.update(|app| app.windows().len()), 2);
}

#[test]
fn a_tab_asking_a_question_stays() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((book, _)) = first(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    blank(&mut cx, &book);
    let id = cx.update(|app| book.read(app).core.active()).expect("a tab");
    // Its first sheet read lands before the question.
    wait(&mut cx, |app| std::rc::Rc::strong_count(&book.read(app).tab(id).expect("the tab").busy) == 1);
    update(&mut cx, &book, |book, _| book.core.dialog = Some(Dialog::Unsaved { tab: id, after: After::Tab }));
    let stays = cx.update(|app| book.update(app, |book, cx| book.move_out(id, cx).err()));
    assert_eq!(stays, Some("office.move.dialog"));
    update(&mut cx, &book, |book, _| book.core.dialog = None);
    let stays = cx.update(|app| book.update(app, |book, cx| book.move_out(id + 100, cx).err()));
    assert_eq!(stays, Some("office.move.loading"), "no such tab is a loading one");
    assert_eq!(cx.update(|app| book.read(app).core.tabs.len()), 1, "nothing left the window");
}

#[test]
fn a_source_check_in_flight_lands_in_the_new_window() {
    use ely_office::{Linked, Linking as _};
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((book, _)) = first(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let path = dir.path().join("source.fods");
    let saved = super::sources::reader(&mut cx, &book, dir.path(), &[&path]);
    let id = super::sources::opened(&mut cx, &book, &saved);
    update(&mut cx, &book, |book, cx| book.update_links(id, cx));
    wait(&mut cx, |app| book.read(app).tab(id).is_some_and(|tab| *tab.links.state() == Linked::Updated(Vec::new())));
    super::sources::source(&path, 5);
    // The check starts here; the tab leaves before it ends.
    cx.update(|app| {
        book.update(app, |book, cx| book.sources_moved(id, None, cx));
        ely_office::moving::to_new_window(book.clone(), id, app);
    });
    wait(&mut cx, |app| book.read(app).tab(id).is_none() && app.windows().len() == 2);
    let other = front(&mut cx);
    let changed = Linked::Changed(path.display().to_string());
    wait(&mut cx, |app| other.read(app).tab(id).is_some_and(|tab| *tab.links.state() == changed));
}

#[test]
fn reads_in_flight_follow_their_workbook() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((book, _)) = first(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    blank(&mut cx, &book);
    update(&mut cx, &book, |book, cx| book.new_blank(cx));
    wait(&mut cx, |app| book.read(app).core.tabs.len() == 2 && book.read(app).active_tab().is_some_and(|tab| tab.extra.read.is_some()));
    let id = cx.update(|app| book.read(app).core.active()).expect("a tab");
    // One update: the move comes before the three reads run.
    cx.update(|app| {
        book.update(app, |book, cx| {
            let view = book.tab(id).expect("the tab").view.clone();
            let extra = &mut book.tab_mut(id).expect("the tab").extra;
            (extra.read, extra.linked, extra.ruled) = (None, None, None);
            book.observed(id, &view, cx);
            let extra = &book.tab(id).expect("the tab").extra;
            assert!(extra.reading && extra.linking && extra.ruling, "sheet, link and rule reads are asked");
        });
        ely_office::moving::to_new_window(book.clone(), id, app);
    });
    wait(&mut cx, |app| book.read(app).tab(id).is_none());
    let other = front(&mut cx);
    assert_ne!(other, book, "a new window is in front");
    wait(&mut cx, |app| {
        other.read(app).tab(id).is_some_and(|tab| {
            let extra = &tab.extra;
            !extra.reading && !extra.linking && !extra.ruling && extra.read.is_some() && extra.linked.is_some() && extra.ruled.is_some()
        })
    });
}

#[test]
fn work_running_for_a_tab_keeps_it_until_it_ends() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((book, _)) = first(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    blank(&mut cx, &book);
    let id = cx.update(|app| book.read(app).core.active()).expect("a tab");
    let held = cx.update(|app| book.read(app).hold(id)).expect("the tab");
    let stays = cx.update(|app| book.update(app, |book, cx| book.move_out(id, cx).err()));
    assert_eq!(stays, Some("office.move.busy"));
    drop(held);
    // A read that fails still lets go of the tab.
    update(&mut cx, &book, |book, cx| book.read_in(id, |_, _| Err::<(), _>(std::io::Error::other("refused")), |_, _, _, _| {}, cx));
    wait(&mut cx, |app| std::rc::Rc::strong_count(&book.read(app).tab(id).expect("the tab").busy) == 1);
    let left = cx.update(|app| book.update(app, |book, cx| book.move_out(id, cx).is_ok()));
    assert!(left, "once the work ends, the tab may move");
}

#[test]
fn an_open_file_panel_keeps_its_tab() {
    use ely_office::Closing as _;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((book, _)) = first(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    blank(&mut cx, &book);
    let id = cx.update(|app| book.read(app).core.active()).expect("a tab");
    update(&mut cx, &book, |book, cx| book.pick(false, cx, |_, _, _| {}));
    assert!(cx.did_prompt_for_paths(), "the panel is open");
    let stays = cx.update(|app| book.update(app, |book, cx| book.move_out(id, cx).err()));
    assert_eq!(stays, Some("office.move.busy"), "the pick may be for this tab");
    cx.simulate_path_prompt_response(|_| None);
    wait(&mut cx, |app| std::rc::Rc::strong_count(&book.read(app).tab(id).expect("the tab").busy) == 1);
    let left = cx.update(|app| book.update(app, |book, cx| book.move_out(id, cx).is_ok()));
    assert!(left, "once the panel closes, the tab may move");
}

/// A blank tab saved as `book.xlsx`, all its work ended.
fn saved(cx: &mut TestAppContext, book: &Entity<Book>, home: AnyWindowHandle, dir: &Path) -> (u64, std::path::PathBuf) {
    use ely_office::Saving as _;
    blank(cx, book);
    let id = cx.update(|app| book.read(app).core.active()).expect("a tab");
    let path = dir.join("book.xlsx");
    home.update(cx, |_, window, app| book.update(app, |book, cx| book.write_as(id, path.clone(), None, window, cx))).expect("home is open");
    wait(cx, |app| book.read(app).tab(id).is_some_and(|tab| tab.session.path().is_some() && !tab.exporting && std::rc::Rc::strong_count(&tab.busy) == 1));
    (id, path)
}

/// Whether tab `id` may leave now; it stays put.
fn leaves(cx: &mut TestAppContext, book: &Entity<Book>, id: u64) -> Option<&'static str> {
    cx.update(|app| book.update(app, |book, cx| book.move_out(id, cx).err()))
}

#[test]
fn save_as_and_keep_mine_keep_their_tab() {
    use ely_office::Saving as _;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((book, home)) = first(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let (id, _) = saved(&mut cx, &book, home, dir.path());
    home.update(&mut cx, |_, window, app| book.update(app, |book, cx| book.save_as(id, None, window, cx))).expect("home is open");
    assert!(cx.did_prompt_for_new_path(), "the save panel is open");
    assert_eq!(leaves(&mut cx, &book, id), Some("office.move.busy"), "the name may be for this tab");
    cx.simulate_new_path_selection(|_| None);
    wait(&mut cx, |app| std::rc::Rc::strong_count(&book.read(app).tab(id).expect("the tab").busy) == 1);
    // Keep Mine reads the disk first; the tab waits.
    let stays = home.update(&mut cx, |_, window, app| book.update(app, |book, cx| {
        book.keep_mine(id, None, window, cx);
        book.move_out(id, cx).err()
    }));
    assert!(stays.expect("home is open").is_some(), "keep mine lands here");
}

#[test]
fn a_program_save_keeps_its_tab_and_is_answered() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((book, home)) = first(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let (id, path) = saved(&mut cx, &book, home, dir.path());
    let request = serde_json::json!(["save", path.display().to_string()]).to_string();
    let (ask, answer) = ely_ui::Ask::for_tests(&request);
    let stays = cx.update(|app| book.update(app, |book, cx| {
        book.answer(ask, cx);
        book.move_out(id, cx).err()
    }));
    assert!(stays.is_some(), "an accepted save keeps its tab");
    wait(&mut cx, |_| answer.try_recv().is_ok_and(|saved| saved.is_ok()));
}
