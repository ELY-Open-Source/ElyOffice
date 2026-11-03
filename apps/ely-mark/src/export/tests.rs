use gpui::TestAppContext;

use crate::desk::tests::{open, start};
use crate::output::Format;

#[gpui::test]
fn export_asks_for_a_format_then_writes_it(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("note.md");
    std::fs::write(&path, "# Note\n\nText $x$.\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    cx.dispatch_action(ely_ui::commands::Export);
    cx.run_until_parked();
    let (tab, format) = desk.read_with(cx, |desk, _| desk.exporting.expect("the dialog"));
    assert_eq!(format, Format::Page);
    let out = files.path().join("note.html");
    desk.update(cx, |desk, cx| {
        desk.exporting = None;
        desk.export_to(tab, out.clone(), Format::Page, cx)
    });
    cx.run_until_parked();
    let html = std::fs::read_to_string(&out).expect("written");
    assert!(
        html.contains("<h1 id=\"h-0\">Note</h1>") && html.contains("<svg"),
        "{html}"
    );
}

#[gpui::test]
fn export_never_writes_over_the_open_file(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("note.md");
    std::fs::write(&path, "# Note\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let tab = desk.read_with(cx, |desk, _| desk.core.active().expect("a tab"));
    desk.update(cx, |desk, cx| {
        desk.export_to(tab, path.clone(), Format::Text, cx)
    });
    cx.run_until_parked();
    assert_eq!(std::fs::read_to_string(&path).expect("reads"), "# Note\n");
}

#[gpui::test]
fn rich_copy_takes_the_selection_as_html(cx: &mut TestAppContext) {
    if std::env::var_os("ELY_CLIPBOARD_TEST").is_none() {
        eprintln!("SKIP: set ELY_CLIPBOARD_TEST=1; this replaces the clipboard");
        return;
    }
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("note.md");
    std::fs::write(&path, "# Note\n\n**Bold** words.\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let view = desk.read_with(cx, |desk, _| desk.active_view().expect("a tab"));
    view.update_in(cx, |view, window, cx| {
        view.switch(crate::view::Mode::Source, window, cx)
    });
    cx.run_until_parked();
    let document = view.read_with(cx, |view, _| view.document().clone());
    document.update(cx, |document, cx| {
        document
            .editor()
            .update(cx, |editor, cx| editor.select(std::iter::once(8..22), cx))
    });
    cx.dispatch_action(crate::export::CopyRich);
    cx.run_until_parked();
    let clip = ely_platform::clipboard::read()
        .expect("reads")
        .expect("a clip");
    let ely_platform::clipboard::Clip::Html { html, text } = clip else {
        panic!("not HTML: {clip:?}");
    };
    assert_eq!(text, "**Bold** words");
    assert!(
        html.contains("<strong>Bold</strong>") && !html.contains("Note"),
        "{html}"
    );
}

#[test]
fn each_print_job_writes_its_own_file() {
    let work = std::path::Path::new("work");
    assert_ne!(super::print_copy(work), super::print_copy(work));
}
