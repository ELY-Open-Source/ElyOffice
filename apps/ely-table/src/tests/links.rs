//! Clicked links and closed kernel dialogs reach the user (#57).

use std::path::Path;

use ely_gpui_component::primitives::Severity;

use super::*;

#[test]
fn links_jump_or_refuse_and_dialogs_say_why() {
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
    // A copy: the kernel locks what it opens.
    let path = dir.path().join("links.xlsx");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/table/links.xlsx");
    std::fs::copy(fixture, &path).expect("copies");
    update(&mut cx, &book, |book, cx| book.open_file(path.clone(), cx));
    settle(&cx, |app| book.read(app).active_tab().is_some_and(|tab| tab.extra.sheets.list.len() == 2));
    let view = cx.update(|app| book.read(app).active_tab().expect("the tab").view.clone());
    let toasted = |app: &gpui::App, want: &str| root.read(app).toasts(app).iter().any(|(text, severity)| *severity == Some(Severity::Danger) && text.as_ref() == want);
    view.update(&mut cx, |_, cx| cx.emit(ely_lok::Linked("#'历史严重违法失信'.B2".into())));
    settle(&cx, |app| {
        let tab = book.read(app).active_tab().expect("the tab");
        tab.extra.sheets.current == 1 && tab.view.read(app).cell().address == "B2"
    });
    view.update(&mut cx, |_, cx| cx.emit(ely_lok::Linked("file:///etc/passwd".into())));
    settle(&cx, |app| toasted(app, &ely_office::text(app, "office.link.refused", &[])));
    let closed = ely_lok::Closed { title: "Information".into(), body: "Invalid range".into() };
    view.update(&mut cx, |_, cx| cx.emit(closed));
    settle(&cx, |app| {
        let said = ely_office::text(app, "office.error.dialog_said", &[("title", "Information"), ("body", "Invalid range")]);
        toasted(app, &said)
    });
}
