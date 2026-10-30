//! Undo and Redo reach the kernel from the window (#15).

use gpui::VisualTestContext;

use super::*;

struct Shown(Entity<Book>);

impl gpui::Render for Shown {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        self.0.clone()
    }
}

/// A drawn Book, its actions live.
fn drawn<'a>(cx: &'a mut TestAppContext, dir: &Path) -> Option<(Entity<Book>, &'a mut VisualTestContext)> {
    let install = ely_lok::test_install()?;
    let worker = ely_office::lok::test_worker(&install, &dir.join("profile"));
    let work = dir.join("work");
    std::fs::create_dir_all(&work).expect("the work dir");
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        ely_ui::settings::install_defaults(cx);
        ely_ui::i18n::install_as("en-US", cx);
        let edit = gpui::Menu::new("Edit").items(ely_office::undo_items((false, false), cx));
        let menus = ely_ui::commands::menus(ely_platform::AppId::Table, &ely_ui::commands::FILE, true, vec![edit], cx);
        cx.set_menus(menus);
        ely_office::bind_undo_keys(cx);
    });
    let (shown, cx) = cx.add_window_view(|window, cx| {
        let book = cx.new(|cx| Book::new(worker, work, window, cx));
        book.update(cx, |book, _| book.core.window = Some(window.window_handle()));
        Shown(book)
    });
    let book = cx.update(|_, app| shown.read(app).0.clone());
    Some((book, cx))
}

#[test]
fn undo_and_redo_from_the_window_step_the_kernel() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((book, cx)) = drawn(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(cx, &book);
    let first = cx.update(|_, app| sheet_names(app, &book));
    fresh(cx, &book);
    update(cx, &book, |book, cx| book.run_active("sheet.rename", &["Plan", "1"], cx));
    settle(cx, |app| sheet_names(app, &book) == ["Plan"]);
    cx.update(|window, app| {
        let focus = book.read(app).core.focus.clone();
        window.focus(&focus, app);
    });
    fresh(cx, &book);
    cx.dispatch_action(ely_office::Undo);
    settle(cx, |app| sheet_names(app, &book) == first);
    fresh(cx, &book);
    cx.dispatch_action(ely_office::Redo);
    settle(cx, |app| sheet_names(app, &book) == ["Plan"]);
    fresh(cx, &book);
    update(cx, &book, |book, cx| book.run_active("sheet.rename", &["Q", "1"], cx));
    settle(cx, |app| sheet_names(app, &book) == ["Q"]);
    // The grid has the keys: one press, one step back.
    cx.update(|window, app| {
        let view = book.read(app).active_tab().expect("a tab").view.clone();
        let focus = view.read(app).focus_handle().clone();
        window.focus(&focus, app);
    });
    fresh(cx, &book);
    cx.simulate_keystrokes("secondary-z");
    settle(cx, |app| sheet_names(app, &book) == ["Plan"]);
    fresh(cx, &book);
    assert_eq!(cx.update(|_, app| sheet_names(app, &book)), ["Plan"], "one step");
}

/// Whether the Edit menu offers Undo and Redo now.
fn offered(app: &gpui::App) -> (bool, bool) {
    let menus = app.get_menus().expect("menus are set");
    let on = |name: &str| {
        menus.iter().flat_map(|menu| &menu.items).any(|item| {
            matches!(item, gpui::OwnedMenuItem::Action { name: shown, disabled: false, .. } if shown == name)
        })
    };
    (on("Undo"), on("Redo"))
}

#[test]
fn undo_and_redo_are_offered_only_when_the_kernel_can() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((book, cx)) = drawn(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(cx, &book);
    fresh(cx, &book);
    let errors = std::rc::Rc::new(std::cell::Cell::new(0));
    cx.update(|_, app| {
        let view = book.read(app).active_tab().expect("a tab").view.clone();
        let errors = errors.clone();
        app.subscribe(&view, move |_, _: &ely_lok::LokError, _| errors.set(errors.get() + 1)).detach();
    });
    cx.run_until_parked();
    assert_eq!(cx.update(|_, app| offered(app)), (false, false), "an empty history");
    // A key press with nothing to undo does nothing, quietly.
    update(cx, &book, |book, cx| book.run_active("edit.undo", &[], cx));
    fresh(cx, &book);
    assert_eq!(errors.get(), 0, "nothing to undo is no error");
    update(cx, &book, |book, cx| book.run_active("sheet.rename", &["Plan", "1"], cx));
    settle(cx, |app| offered(app) == (true, false));
    cx.dispatch_action(ely_office::Undo);
    settle(cx, |app| offered(app).1);
    assert_eq!(errors.get(), 0);
}
