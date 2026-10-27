use std::path::Path;

use gpui::{AppContext as _, Entity, TestAppContext, Window};

use crate::docs::Docs;
use crate::files::Template;
use ely_office::prelude::*;
use ely_office::{After, Dialog};

struct Blank;

impl gpui::Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        gpui::Empty
    }
}

/// A Docs over a fresh worker, if LOK exists.
pub(crate) fn docs(cx: &mut TestAppContext, dir: &Path) -> Option<Entity<Docs>> {
    // A failing test prints its log.
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Info)
        .try_init();
    let install = ely_lok::install_dir().ok()?;
    let worker = ely_office::lok::test_worker(&install, &dir.join("profile"));
    let work = dir.join("work");
    std::fs::create_dir_all(&work).expect("the work dir");
    cx.update(|app| {
        ely_ui::settings::install_defaults(app);
        ely_ui::i18n::install(app);
        Some(app.new(|cx| Docs::new(worker, work, cx)))
    })
}

/// Steps the scheduler until `done` or ten seconds pass.
#[track_caller]
pub(crate) fn settle(cx: &TestAppContext, done: impl Fn(&gpui::App) -> bool) {
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

/// Waits until `path` holds bytes other than `before`.
#[track_caller]
pub(crate) fn written(cx: &TestAppContext, path: &Path, before: Option<&[u8]>) {
    settle(cx, |_| {
        std::fs::read(path).is_ok_and(|bytes| Some(bytes.as_slice()) != before)
    });
}

/// Runs `call` with a window, as actions get one.
pub(crate) fn with_window(
    cx: &mut TestAppContext,
    docs: &Entity<Docs>,
    call: impl FnOnce(&mut Docs, &mut Window, &mut gpui::Context<Docs>),
) {
    let window = cx.add_window(|_, _| Blank);
    cx.update_window(window.into(), |_, window, app| {
        docs.update(app, |docs, cx| {
            docs.core.window = Some(window.window_handle());
            call(docs, window, cx)
        })
    })
    .expect("the window lives");
}

pub(crate) fn type_into(cx: &mut TestAppContext, docs: &Entity<Docs>, text: &str) {
    let view = cx.update(|app| docs.read(app).active_tab().expect("a tab").view.clone());
    let window = cx.add_window(|_, _| Blank);
    cx.update_window(window.into(), |_, window, app| {
        let mut ime = gpui::ElementInputHandler::new(gpui::Bounds::default(), view.clone());
        gpui::InputHandler::replace_text_in_range(&mut ime, None, text, window, app);
    })
    .expect("the window lives");
    settle(cx, |app| {
        docs.read(app)
            .active_tab()
            .is_some_and(|tab| tab.dirty(app) && tab.view.read(app).settled())
    });
}

#[test]
fn a_new_document_saves_as_docx_and_comes_back_clean() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    with_window(&mut cx, &docs, |docs, _, cx| {
        docs.new_from(Template::Report, cx)
    });
    settle(&cx, |app| docs.read(app).core.active.is_some());
    let (id, dirty) = cx.update(|app| {
        let tab = docs.read(app).active_tab().expect("a tab");
        (tab.id, tab.dirty(app))
    });
    assert!(!dirty, "a fresh template is clean");
    type_into(&mut cx, &docs, "Ely");
    let target = dir.path().join("Report.docx");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, target.clone(), None, window, cx)
    });
    written(&cx, &target, None);
    let bytes = std::fs::read(&target).expect("the docx exists");
    assert!(bytes.starts_with(b"PK"), "a docx zip");
    settle(&cx, |app| {
        docs.read(app)
            .active_tab()
            .is_some_and(|tab| !tab.dirty(app))
    });
    cx.update(|app| {
        let tab = docs.read(app).active_tab().expect("a tab");
        assert_eq!(tab.name.as_ref(), "Report.docx");
        assert!(tab.registry.is_some(), "the saved file joins the registry");
    });
}

#[test]
fn a_save_keeps_the_format_and_save_as_converts_to_a_new_file() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let odt = dir.path().join("Notes.odt");
    with_window(&mut cx, &docs, |docs, _, cx| {
        docs.new_from(Template::Blank, cx)
    });
    settle(&cx, |app| docs.read(app).core.active.is_some());
    let id = cx.update(|app| docs.read(app).core.active.expect("a tab"));
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, odt.clone(), None, window, cx)
    });
    written(&cx, &odt, None);
    type_into(&mut cx, &docs, "more");
    let first = std::fs::read(&odt).expect("the first odt");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.save(id, None, window, cx)
    });
    written(&cx, &odt, Some(&first));
    let odf = std::fs::read(&odt).expect("the odt exists");
    let mime = b"application/vnd.oasis.opendocument.text";
    assert!(
        odf.windows(mime.len()).any(|w| w == mime),
        "the save kept ODT"
    );
    let docx = dir.path().join("Notes.docx");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, docx.clone(), None, window, cx)
    });
    written(&cx, &docx, None);
    assert!(std::fs::read(&docx).expect("the docx").starts_with(b"PK"));
    assert_eq!(
        std::fs::read(&odt).expect("the odt stays"),
        odf,
        "the source is untouched"
    );
    let pdf = dir.path().join("Notes.pdf");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, pdf.clone(), None, window, cx)
    });
    assert!(!pdf.exists(), "Save As writes only DOCX or ODT");
}

#[test]
fn a_local_template_opens_untitled_and_stays_untouched() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let template = dir.path().join("memo.fodt");
    let fodt = cx.update(|app| Template::Letter.fodt(app));
    std::fs::write(&template, &fodt).expect("the template writes");
    with_window(&mut cx, &docs, |docs, _, cx| {
        docs.new_from_file(&template, cx)
    });
    settle(&cx, |app| docs.read(app).core.active.is_some());
    type_into(&mut cx, &docs, "x");
    let id = cx.update(|app| {
        let tab = docs.read(app).active_tab().expect("a tab");
        assert!(tab.session.path().is_none(), "a template opens untitled");
        tab.id
    });
    let out = dir.path().join("memo.docx");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, out.clone(), None, window, cx)
    });
    written(&cx, &out, None);
    assert!(out.exists());
    assert_eq!(
        std::fs::read_to_string(&template).expect("the template"),
        fodt,
        "the original stays"
    );
}

#[test]
fn an_open_file_gets_focus_and_unsaved_closes_ask() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let file = dir.path().join("open.fodt");
    let fodt = cx.update(|app| Template::Report.fodt(app));
    std::fs::write(&file, fodt).expect("the file writes");
    with_window(&mut cx, &docs, |docs, _, cx| {
        docs.open_file(file.clone(), cx)
    });
    settle(&cx, |app| docs.read(app).core.tabs.len() == 1);
    with_window(&mut cx, &docs, |docs, _, cx| {
        docs.open_file(file.clone(), cx)
    });
    for _ in 0..40 {
        cx.executor().tick();
    }
    assert_eq!(
        cx.update(|app| docs.read(app).core.tabs.len()),
        1,
        "the same file focuses"
    );
    type_into(&mut cx, &docs, "edit");
    let id = cx.update(|app| docs.read(app).core.active.expect("a tab"));
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.request_close(id, After::Tab, window, cx)
    });
    cx.update(|app| {
        let asking =
            matches!(docs.read(app).core.dialog, Some(Dialog::Unsaved { tab, .. }) if tab == id);
        assert!(asking, "a dirty close asks");
    });
    assert!(
        cx.update(|app| docs.read(app).core.tabs.len()) == 1,
        "nothing closed yet"
    );
}

/// The text the kernel reads back from `path`.
pub(crate) fn text_of(cx: &mut TestAppContext, docs: &Entity<Docs>, path: &Path) -> String {
    let worker = cx.update(|app| docs.read(app).core.worker.clone());
    let mut worker = worker.lock().expect("the worker lock");
    let doc = worker
        .load(&ely_lok::file_url(path))
        .expect("the saved file loads");
    let flat = path.with_extension("check.fodt");
    doc.save_as(&mut worker, &ely_lok::file_url(&flat), "fodt")
        .expect("flattens");
    worker.close_doc(doc).expect("closes");
    std::fs::read_to_string(&flat).expect("reads")
}

/// Waits for the active tab's queued input to land.
pub(crate) fn settled(cx: &TestAppContext, docs: &Entity<Docs>) {
    settle(cx, |app| {
        let tab = docs.read(app).active_tab().expect("a tab");
        tab.view.read(app).settled()
    });
}

/// Opens the blank template and returns its tab id.
pub(crate) fn blank(cx: &mut TestAppContext, docs: &Entity<Docs>) -> u64 {
    let before = cx.update(|app| docs.read(app).core.tabs.len());
    with_window(cx, docs, |docs, _, cx| docs.new_from(Template::Blank, cx));
    settle(cx, |app| docs.read(app).core.tabs.len() > before);
    cx.update(|app| docs.read(app).core.tabs.last().expect("a tab").id)
}

/// Commits `text` through the view without waiting.
pub(crate) fn commit(cx: &mut TestAppContext, docs: &Entity<Docs>, text: &str) {
    let view = cx.update(|app| docs.read(app).active_tab().expect("a tab").view.clone());
    let window = cx.add_window(|_, _| Blank);
    cx.update_window(window.into(), |_, window, app| {
        let mut ime = gpui::ElementInputHandler::new(gpui::Bounds::default(), view.clone());
        gpui::InputHandler::replace_text_in_range(&mut ime, None, text, window, app);
    })
    .expect("the window lives");
}

#[test]
fn queued_input_reaches_the_save_and_the_close_prompt() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    commit(&mut cx, &docs, "PendingText");
    let out = dir.path().join("pending.docx");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, out.clone(), None, window, cx)
    });
    written(&cx, &out, None);
    settle(&cx, |_| out.exists());
    assert!(
        text_of(&mut cx, &docs, &out).contains("PendingText"),
        "the queued text saved"
    );
    commit(&mut cx, &docs, "More");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.request_close(id, After::Tab, window, cx)
    });
    settle(&cx, |app| {
        matches!(docs.read(app).core.dialog, Some(Dialog::Unsaved { .. }))
    });
}

#[test]
fn a_macro_file_opens_in_risk_mode_and_saves_only_as_a_copy() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let macro_doc = dir.path().join("macro.fodt");
    let body = cx.update(|app| Template::Report.fodt(app))
        .replace(
            "office:version=",
            "xmlns:script=\"urn:oasis:names:tc:opendocument:xmlns:script:1.0\" xmlns:ooo=\"http://openoffice.org/2004/office\" office:version=",
        )
        .replace(
            "<office:body>",
            "<office:scripts><office:script script:language=\"ooo:Basic\"><ooo:libraries><ooo:library-embedded ooo:name=\"Standard\"><ooo:module ooo:name=\"Module1\"><ooo:source-code>Sub Main&#x0A;End Sub</ooo:source-code></ooo:module></ooo:library-embedded></ooo:libraries></office:script></office:scripts><office:body>",
        );
    std::fs::write(&macro_doc, &body).expect("the macro file writes");
    with_window(&mut cx, &docs, |docs, _, cx| {
        docs.open_file(macro_doc.clone(), cx)
    });
    settle(&cx, |app| docs.read(app).core.active.is_some());
    let id = cx.update(|app| {
        let tab = docs.read(app).active_tab().expect("a tab");
        assert!(tab.risk.is_some(), "macros put the tab in risk mode");
        tab.id
    });
    type_into(&mut cx, &docs, "x");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, macro_doc.with_extension("docx"), None, window, cx)
    });
    let copy = macro_doc.with_extension("docx");
    written(&cx, &copy, None);
    assert!(copy.exists(), "the copy writes");
    assert_eq!(
        std::fs::read_to_string(&macro_doc).expect("source"),
        body,
        "the source stays"
    );
    cx.update(|app| {
        assert!(
            docs.read(app).active_tab().expect("a tab").risk.is_none(),
            "the DOCX copy holds no macros"
        )
    });
}

#[cfg(unix)]
#[test]
fn an_alias_never_converts_the_open_file() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    let source = dir.path().join("source.odt");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, source.clone(), None, window, cx)
    });
    written(&cx, &source, None);
    let before = std::fs::read(&source).expect("the odt");
    let alias = dir.path().join("alias.docx");
    std::os::unix::fs::symlink(&source, &alias).expect("the alias");
    type_into(&mut cx, &docs, "y");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, alias.clone(), None, window, cx)
    });
    written(&cx, &source, Some(&before));
    let after = std::fs::read(&source).expect("the odt");
    let has = |bytes: &[u8], mark: &[u8]| bytes.windows(mark.len()).any(|w| w == mark);
    assert!(
        has(&after, b"application/vnd.oasis.opendocument.text"),
        "the file stays ODT"
    );
    assert!(
        !has(&after, b"word/document.xml"),
        "no DOCX lands in the ODT"
    );
    assert_ne!(after, before, "the alias saved in the file's own format");
    let link = dir.path().join("link.docx");
    std::fs::hard_link(&source, &link).expect("the hard link");
    type_into(&mut cx, &docs, "z");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, link.clone(), None, window, cx)
    });
    assert_eq!(
        std::fs::read(&source).expect("the odt"),
        after,
        "a hard link never converts the open file"
    );
    assert_eq!(
        std::fs::read(&link).expect("the link"),
        after,
        "the link is refused, not replaced"
    );
}

#[test]
fn an_outside_change_asks_and_keep_mine_overwrites() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    let own = dir.path().join("own.docx");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, own.clone(), None, window, cx)
    });
    written(&cx, &own, None);
    type_into(&mut cx, &docs, "Mine");
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&own, "EXTERNAL REVISION").expect("the outside write");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.save(id, None, window, cx)
    });
    settle(&cx, |app| {
        matches!(docs.read(app).core.dialog, Some(Dialog::Conflict { .. }))
    });
    assert_eq!(
        std::fs::read_to_string(&own).expect("own"),
        "EXTERNAL REVISION"
    );
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.keep_mine(id, None, window, cx)
    });
    written(&cx, &own, Some(b"EXTERNAL REVISION"));
    assert!(
        text_of(&mut cx, &docs, &own).contains("Mine"),
        "keep mine wrote the local version"
    );
}

pub(crate) mod find;
mod more;
mod overlap;
mod points;
mod remote;

impl Docs {
    /// Chooses the active tab as template, then types a field.
    pub(crate) fn field_here(&mut self, header: &str, cx: &mut gpui::Context<Self>) {
        let active = self.core.active;
        self.merge.update(cx, |merge, _| merge.source = active);
        self.insert_field(header, cx);
    }
}
