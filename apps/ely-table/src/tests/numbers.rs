//! Sheet numbers hold only in the state they were read.

use super::*;

#[test]
fn a_rename_asked_before_a_delete_renames_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &book);
    for (name, at) in [("B", "2"), ("C", "3")] {
        fresh(&cx, &book);
        update(&mut cx, &book, |book, cx| book.run_active("sheet.insert", &[name, at], cx));
    }
    settle(&cx, |app| sheet_names(app, &book).len() == 3);
    fresh(&cx, &book);
    update(&mut cx, &book, |book, cx| book.show_sheet(1, cx));
    cx.update_window(window.into(), |_, window, app| book.update(app, |book, cx| book.ask_rename(window, cx)))
        .expect("the window lives");
    // The first sheet goes while the dialog is open.
    update(&mut cx, &book, |book, cx| book.run_active("sheet.delete", &["1"], cx));
    settle(&cx, |app| sheet_names(app, &book) == ["B", "C"]);
    fresh(&cx, &book);
    update(&mut cx, &book, |book, cx| book.rename_asked("Renamed B", cx));
    fresh(&cx, &book);
    assert_eq!(cx.update(|app| sheet_names(app, &book)), ["B", "C"], "C keeps its name");
}

#[test]
fn a_failed_command_still_moves_the_state_on() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &book);
    // No merged cell to split: the kernel refuses.
    update(&mut cx, &book, |book, cx| book.run_active("cells.split", &[], cx));
    fresh(&cx, &book);
    update(&mut cx, &book, |book, cx| book.run_active("sheet.rename", &["Plan", "1"], cx));
    settle(&cx, |app| sheet_names(app, &book) == ["Plan"]);
}

#[test]
fn a_failed_print_says_printing_failed() {
    use ely_gpui_component::primitives::Severity;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    // The window's toasts live on its root.
    let root = window
        .update(&mut cx, |_, window, cx| {
            let root = ely_ui::document_root(ely_platform::AppId::Table, book.clone().into(), &ely_ui::commands::FILE, Vec::new(), window, cx);
            book.update(cx, |book, _| book.core.root = root.downgrade());
            root
        })
        .expect("the window");
    let gone = dir.path().join("gone.pdf");
    update(&mut cx, &book, |_, cx| ely_office::print_later(gone, cx));
    // The file is read on its own thread.
    settle(&cx, |app| {
        let prefix = ely_office::text(app, "office.print.failed", &[("error", "")]);
        root.read(app).toasts(app).iter().any(|(text, severity)| *severity == Some(Severity::Danger) && text.starts_with(prefix.as_ref()))
    });
}

/// The whole book drawn, its dialogs too.
#[cfg(windows)]
struct Whole(gpui::Entity<crate::book::Book>);

#[cfg(windows)]
impl gpui::Render for Whole {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};
        gpui::div().size_full().child(self.0.clone())
    }
}

#[cfg(windows)]
#[test]
fn a_windows_print_shows_its_pages_then_prints() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let Some((_, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let window = cx.add_window(|_, _| Whole(book.clone()));
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/pdf/sizes.pdf");
    let (pdf, printed) = (dir.path().join("print.pdf"), dir.path().join("printed.pdf"));
    std::fs::copy(fixture, &pdf).expect("the print copy");
    // The file printer writes the job; no dialog asks.
    unsafe { std::env::set_var("ELY_PRINT_TO", &printed) };
    update(&mut cx, &book, |_, cx| ely_office::print_later(pdf.clone(), cx));
    settle(&cx, |app| matches!(&book.read(app).core.dialog, Some(ely_office::Dialog::Print { pages, .. }) if pages.len() == 2));
    let mut shown = gpui::VisualTestContext::from_window(window.into(), &cx);
    // The pictures load off the app thread; frames draw them.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let at = loop {
        shown.update(|window, _| window.refresh());
        shown.run_until_parked();
        if let Some(bounds) = shown.debug_bounds("print.print") {
            break bounds.center();
        }
        assert!(std::time::Instant::now() < deadline, "the preview never drew");
        std::thread::sleep(std::time::Duration::from_millis(2));
    };
    shown.simulate_click(at, gpui::Modifiers::none());
    settle(&cx, |_| printed.metadata().is_ok_and(|file| file.len() > 0));
    settle(&cx, |app| book.read(app).core.dialog.is_none() && !pdf.exists() && !pdf.with_extension("pages").exists());
}

#[test]
fn a_color_state_reads_as_mixed_auto_or_a_color() {
    use crate::format::paint;
    use ely_lok::show::Paint;
    assert_eq!(paint(Some(ely_lok::MIXED)), Paint::Mixed);
    assert_eq!(paint(Some("16711680")), Paint::Color(0xFF0000));
    assert_eq!(paint(Some("-1")), Paint::None);
    assert_eq!(paint(None), Paint::None);
}
