use super::*;
use ely_platform::clipboard::{self, Clip};
use gpui::TestAppContext;

/// Both tests own the system clipboard.
static CLIPBOARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

const BOOK: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
 xmlns:of="urn:oasis:names:tc:opendocument:xmlns:of:1.2" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="S"><table:table-row>
 <table:table-cell office:value-type="float" office:value="2"><text:p>2</text:p></table:table-cell>
 <table:table-cell table:formula="of:=[.A1]*3" office:value-type="float" office:value="6"><text:p>6</text:p></table:table-cell>
 </table:table-row></table:table></office:spreadsheet></office:body>
</office:document>"#;

fn flush(cx: &TestAppContext) {
    for _ in 0..80 {
        cx.executor().tick();
    }
}

fn goto(view: &Entity<LokView>, cx: &mut TestAppContext, cell: &str) {
    cx.update(|app| {
        let view = view.read(app);
        let mut worker = view.worker.lock().expect("the worker lock");
        worker
            .run(&view.doc, "cell.goto", &[cell])
            .expect("the cell");
    });
}

/// The workbook as flat XML.
fn saved(view: &Entity<LokView>, cx: &mut TestAppContext) -> String {
    let dir = tempfile::tempdir().expect("a temp dir");
    let out = dir.path().join("out.fods");
    cx.update(|app| {
        let view = view.read(app);
        let mut worker = view.worker.lock().expect("the worker lock");
        view.doc
            .save_as(&mut worker, &crate::file_url(&out), "fods")
            .expect("the save works");
    });
    std::fs::read_to_string(&out).expect("the save reads")
}

#[test]
fn values_and_formulas_paste_alone_from_the_views_copy() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    if std::env::var_os("ELY_CLIPBOARD_TEST").is_none() {
        eprintln!("SKIP: set ELY_CLIPBOARD_TEST=1; this replaces the clipboard");
        return;
    }
    let _clipboard = CLIPBOARD.lock().expect("the clipboard lock");
    let mut cx = TestAppContext::single();
    let (view, _) = cx.update(|app| live_view_of(app, &install, "book.fods", BOOK));
    goto(&view, &mut cx, "B1");
    cx.update(|app| {
        view.update(app, |view, _| {
            view.send(crate::view::input::Input::Copy { cut: false })
        })
    });
    flush(&cx);
    for (cell, id) in [("C1", "paste.values"), ("D1", "paste.formulas")] {
        goto(&view, &mut cx, cell);
        cx.update(|app| view.update(app, |view, _| view.paste_only(id)));
        flush(&cx);
    }
    let xml = saved(&view, &mut cx);
    assert_eq!(xml.matches("table:formula=").count(), 2, "C1 holds 6 alone");
    assert!(xml.contains("of:=[.C1]*3"), "D1 keeps the formula");
    let copy = cx.update(|app| {
        let read = clipboard::read().expect("the clipboard reads");
        let Some(Clip::Html { html, .. }) = read else {
            panic!("the copy lands as html");
        };
        view.read(app).holds_copy(&html)
    });
    assert!(copy, "the view knows its own copy");
    // Copies made elsewhere carry no mark of this view.
    let other = "<table><tr><td>6</td></tr></table>";
    assert!(cx.update(|app| !view.read(app).holds_copy(other)));
    let foreign = "<td>6</td><!--ely-copy:1-999-->";
    assert!(
        cx.update(|app| !view.read(app).holds_copy(foreign)),
        "another copy's mark"
    );
}

#[test]
fn copied_cells_keep_a_still_outline_until_an_edit_or_escape() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    if std::env::var_os("ELY_CLIPBOARD_TEST").is_none() {
        eprintln!("SKIP: set ELY_CLIPBOARD_TEST=1; this replaces the clipboard");
        return;
    }
    let _clipboard = CLIPBOARD.lock().expect("the clipboard lock");
    let mut cx = TestAppContext::single();
    let (view, _) = cx.update(|app| live_view_of(app, &install, "book.fods", BOOK));
    let lit = |cx: &mut TestAppContext| cx.update(|app| view.read(app).ants.is_some());
    let copy = |cx: &mut TestAppContext, cut: bool| {
        goto(&view, cx, "A1");
        goto(&view, cx, "B1");
        // The view hears of B1 before the copy.
        for _ in 0..50 {
            if cx.update(|app| view.read(app).cell.cursor.is_some_and(|(x, ..)| x > 0)) {
                break;
            }
            flush(cx);
        }
        cx.update(|app| view.update(app, |view, _| view.copy(cut)));
        flush(cx);
    };
    copy(&mut cx, false);
    assert!(lit(&mut cx), "a copy outlines its cells");
    goto(&view, &mut cx, "C1");
    cx.update(|app| view.update(app, |view, _| view.paste_only("paste.values")));
    flush(&cx);
    assert!(lit(&mut cx), "a copy's outline outlives its paste");
    let escape = gpui::Keystroke::parse("escape").expect("a keystroke");
    cx.update(|app| view.update(app, |view, _| view.key_down(&escape)));
    assert!(!lit(&mut cx), "Escape puts it out");
    copy(&mut cx, false);
    goto(&view, &mut cx, "E1");
    let typed = crate::view::input::Input::Text { text: "7".into(), ends: true };
    cx.update(|app| view.update(app, |view, _| view.send(typed)));
    flush(&cx);
    let painted = cx.update(|app| view.update(app, |view, _| view.ants()));
    assert!(painted.is_none(), "an edit puts it out");
    copy(&mut cx, true);
    assert!(lit(&mut cx), "a cut outlines its cells");
    goto(&view, &mut cx, "F1");
    cx.update(|app| view.update(app, |view, _| view.paste()));
    assert!(!lit(&mut cx), "a cut's paste puts it out");
}

struct Host(Entity<LokView>);

impl gpui::Render for Host {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        self.0.clone()
    }
}

/// Ticks until `done` holds, at most a minute.
fn until(cx: &mut gpui::VisualTestContext, mut done: impl FnMut(&mut gpui::VisualTestContext) -> bool) -> bool {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while std::time::Instant::now() < deadline {
        if !cx.executor().tick() {
            cx.executor().advance_clock(std::time::Duration::from_millis(20));
        }
        if done(cx) {
            return true;
        }
    }
    false
}

/// The clipboard's plain text, if it holds a copy.
fn copied_text() -> Option<String> {
    match clipboard::read().expect("the clipboard reads") {
        Some(Clip::Html { text, .. } | Clip::Text(text)) => Some(text),
        _ => None,
    }
}

/// The text document's paragraphs, saved flat.
fn paragraphs(view: &Entity<LokView>, cx: &mut gpui::VisualTestContext) -> Vec<String> {
    let dir = tempfile::tempdir().expect("a temp dir");
    let out = dir.path().join("out.fodt");
    cx.update(|_, app| {
        let view = view.read(app);
        let mut worker = view.worker.lock().expect("the worker lock");
        view.doc.save_as(&mut worker, &crate::file_url(&out), "fodt").expect("the save works");
    });
    crate::client::tests::input::paragraphs(&std::fs::read_to_string(&out).expect("the save reads"))
}

#[test]
fn insert_and_delete_chords_use_the_system_clipboard() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    if std::env::var_os("ELY_CLIPBOARD_TEST").is_none() {
        eprintln!("SKIP: set ELY_CLIPBOARD_TEST=1; this replaces the clipboard");
        return;
    }
    let _clipboard = CLIPBOARD.lock().expect("the clipboard lock");
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let (view, _) = cx.update(|app| live_view(app, &install));
    let (_, window) = cx.add_window_view(|_, _| Host(view.clone()));
    window.run_until_parked();
    window.update(|window, app| view.read(app).focus_handle().clone().focus(window, app));
    let select_all = |window: &mut gpui::VisualTestContext| {
        window.update(|_, app| {
            let view = view.read(app);
            let mut worker = view.worker.lock().expect("the worker lock");
            worker.run(&view.doc, "edit.select_all", &[]).expect("select all");
        });
    };
    let before = || clipboard::write(&Clip::Text("before".into())).expect("the clipboard writes");
    before();
    select_all(window);
    window.simulate_keystrokes("ctrl-insert");
    let copied = until(window, |_| copied_text().as_deref() == Some("Hello 世界"));
    assert!(copied, "Ctrl+Insert copied to the system clipboard: {:?}", copied_text());
    window.simulate_keystrokes("end shift-insert");
    let twice = |text: &str| text.matches("Hello 世界").count() == 2;
    let pasted = until(window, |cx| twice(&paragraphs(&view, cx).concat()));
    assert!(pasted, "Shift+Insert pasted the copy: {:?}", paragraphs(&view, window));
    before();
    select_all(window);
    window.simulate_keystrokes("shift-delete");
    let cut = until(window, |_| copied_text().is_some_and(|text| twice(&text)));
    assert!(cut, "Shift+Delete cut to the system clipboard: {:?}", copied_text());
    let gone = until(window, |cx| paragraphs(&view, cx).concat().is_empty());
    assert!(gone, "the cut text is gone: {:?}", paragraphs(&view, window));
}
