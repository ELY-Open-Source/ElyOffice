use std::{cell::RefCell, rc::Rc};

use ely_gpui_component::theme::Theme;
use ely_lok::LokError;
use ely_office::panel::Panel as _;
use gpui::{Entity, TestAppContext};

use super::Props;
use crate::docs::Docs;
use crate::tests::{blank, docs, settle, settled, text_of, type_into, with_window, written};
use ely_office::prelude::*;

/// The panel, shown on the active tab.
pub(super) fn panel(cx: &mut TestAppContext, docs: &Entity<Docs>) -> Entity<Props> {
    let props = cx.update(|app| {
        let tab = docs.read(app).active_tab().expect("a tab");
        let (view, doc) = (tab.view.clone(), tab.doc.clone());
        let props = docs.read(app).props.clone();
        props.update(app, |props, cx| props.show(Some((&view, &doc)), cx));
        props
    });
    // The lists come from a background read.
    settle(cx, |app| {
        let props = props.read(app);
        !props.fonts.is_empty() && props.shown.as_ref().is_some_and(|shown| !shown.paragraph.is_empty())
    });
    props
}

fn run(cx: &mut TestAppContext, props: &Entity<Props>, id: &str, values: &[&str]) {
    cx.update(|app| props.update(app, |props, cx| props.run(id, values, cx)));
}

#[test]
fn the_panel_formats_text_paragraphs_and_styles() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Ely panel");
    let props = panel(&mut cx, &docs);
    cx.update(|app| {
        let props = props.read(app);
        let shown = props.shown.as_ref().expect("shown");
        assert!(!props.fonts.is_empty(), "the kernel lists fonts");
        assert!(shown.paragraph.iter().any(|name| name == "Heading 2"));
        assert!(shown.character.iter().any(|name| name == "Emphasis"));
    });
    for (command, values) in [
        ("edit.select_all", vec![]),
        ("format.style", vec!["Heading 2", "ParagraphStyles"]),
        ("format.color", vec!["16711680"]),
        ("format.highlight", vec!["65535"]),
        ("format.superscript", vec![]),
        ("paragraph.tab", vec!["-1", "1134", "false"]),
        ("list.bullets", vec![]),
    ] {
        run(&mut cx, &props, command, &values);
    }
    cx.update(|app| props.update(app, |props, cx| props.create_style("Ely Note", "2", cx)));
    settle(&cx, |app| {
        let props = props.read(app);
        let shown = props.shown.as_ref().expect("shown");
        !props.reload && shown.paragraph.iter().any(|name| name == "Ely Note")
    });
    run(&mut cx, &props, "paragraph.right", &[]);
    run(&mut cx, &props, "style.update", &["Ely Note", "2"]);
    run(
        &mut cx,
        &props,
        "format.style",
        &["Emphasis", "CharacterStyles"],
    );
    settle(&cx, |app| {
        let tab = docs.read(app).active_tab().expect("a tab");
        let view = tab.view.read(app);
        view.settled() && view.tabs().0.len() == 1 && view.state(".uno:FontColor") == Some("16711680")
    });
    // The shown stop, removed at the revision that read it.
    cx.update(|app| {
        let read = docs.read(app).active_tab().expect("a tab").view.read(app).tabs().1;
        props.update(app, |props, cx| props.run_at(&read, "paragraph.tab", &["0", "0", "true"], cx))
    });
    settle(&cx, |app| {
        let view = docs.read(app).active_tab().expect("a tab").view.read(app);
        view.settled() && view.tabs().0.is_empty()
    });
    run(&mut cx, &props, "paragraph.tab", &["-1", "1134", "false"]);
    let target = dir.path().join("panel.docx");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, target.clone(), None, window, cx)
    });
    written(&cx, &target, None);
    let text = text_of(&mut cx, &docs, &target);
    for mark in [
        "fo:color=\"#ff0000\"",
        "fo:background-color=\"#00ffff\"",
        "style:text-position=\"super",
        "style:tab-stop",
        "<text:list",
        "style:display-name=\"Ely Note\" style:family=\"paragraph\" style:parent-style-name=\"Heading_20_2\"",
        "style:parent-style-name=\"Ely_20_Note\"",
        "text:style-name=\"Emphasis\"",
    ] {
        assert!(text.contains(mark), "{mark} lands");
    }
    let note = text
        .split("style:display-name=\"Ely Note\"")
        .nth(1)
        .and_then(|rest| rest.split("</style:style>").next())
        .expect("the style is written");
    assert!(
        note.contains("fo:text-align=\"right\""),
        "the update reached the style"
    );
}

#[test]
fn bad_values_report_and_the_panel_follows_the_tab() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &docs);
    let props = panel(&mut cx, &docs);
    let errors = Rc::new(RefCell::new(Vec::new()));
    cx.update(|app| {
        let errors = errors.clone();
        app.subscribe(&props, move |_, error: &LokError, _| {
            errors.borrow_mut().push(error.0.clone())
        })
        .detach()
    });
    run(&mut cx, &props, "format.size", &["0"]);
    assert_eq!(errors.borrow().len(), 1, "an out-of-range size reports");
    assert!(errors.borrow()[0].starts_with("format.size"));
    let first = cx.update(|app| docs.read(app).active_tab().expect("a tab").view.entity_id());
    blank(&mut cx, &docs);
    let props = panel(&mut cx, &docs);
    let second = cx.update(|app| docs.read(app).active_tab().expect("a tab").view.entity_id());
    assert_ne!(first, second);
    cx.update(|app| {
        let shown = props
            .read(app)
            .shown
            .as_ref()
            .map(|shown| shown.view.entity_id());
        assert_eq!(shown, Some(second), "the panel moves to the new tab");
        props.update(app, |props, cx| props.show(None, cx));
        assert!(props.read(app).shown.is_none(), "no tab, no panel");
    });
}

/// Posts `keys` (LOK key codes) straight to the active document.
fn keys(cx: &mut TestAppContext, docs: &Entity<Docs>, keys: &[i32]) {
    cx.update(|app| {
        let docs = docs.read(app);
        let doc = docs.active_tab().expect("a tab").doc.clone();
        let mut worker = docs.core.worker.lock().expect("the worker lock");
        for &code in keys {
            worker.post_key(&doc, 0, code).expect("the key posts");
        }
    });
}

/// A text style's definition in a flat ODF file.
fn text_style<'a>(text: &'a str, name: &str) -> &'a str {
    let head = format!("style:name=\"{name}\"");
    text.split(head.as_str())
        .nth(1)
        .and_then(|rest| rest.split("</style:style>").next())
        .unwrap_or_else(|| panic!("{name} is defined"))
}

#[test]
fn update_follows_the_character_style_under_the_cursor() {
    const HOME: i32 = 1028;
    const END: i32 = 1029;
    const LEFT: i32 = 1026;
    const RIGHT: i32 = 1027;
    const SHIFT: i32 = 0x1000;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Alpha Beta");
    let props = panel(&mut cx, &docs);
    run(&mut cx, &props, "edit.select_all", &[]);
    run(
        &mut cx,
        &props,
        "format.style",
        &["Strong Emphasis", "CharacterStyles"],
    );
    settled(&cx, &docs);
    keys(
        &mut cx,
        &docs,
        &[
            HOME,
            RIGHT | SHIFT,
            RIGHT | SHIFT,
            RIGHT | SHIFT,
            RIGHT | SHIFT,
            RIGHT | SHIFT,
        ],
    );
    run(
        &mut cx,
        &props,
        "format.style",
        &["Emphasis", "CharacterStyles"],
    );
    settled(&cx, &docs);
    keys(
        &mut cx,
        &docs,
        &[END, LEFT | SHIFT, LEFT | SHIFT, LEFT | SHIFT, LEFT | SHIFT],
    );
    let current = |app: &gpui::App| props.read(app).state(".uno:CharStyle", app);
    settle(&cx, |app| {
        current(app).as_deref() == Some("Strong Emphasis")
    });
    cx.update(|app| {
        props.update(app, |props, cx| props.show(None, cx));
        assert_eq!(current(app), None, "a hidden panel shows nothing");
    });
    let props = panel(&mut cx, &docs);
    let target = cx.update(|app| props.read(app).state(".uno:CharStyle", app));
    assert_eq!(
        target.as_deref(),
        Some("Strong Emphasis"),
        "shown again, still synced"
    );
    run(&mut cx, &props, "format.color", &["16711680"]);
    let target = target.expect("a reported style");
    run(&mut cx, &props, "style.update", &[&target, "1"]);
    settled(&cx, &docs);
    run(&mut cx, &props, "edit.select_all", &[]);
    settle(&cx, |app| current(app).as_deref() == Some(ely_lok::MIXED));
    let mixed = cx.update(|app| current(app)).expect("reported");
    assert!(
        !ely_office::panel::known(&mixed),
        "a mixed selection has no update target"
    );
    let path = dir.path().join("styles.docx");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, path.clone(), None, window, cx)
    });
    written(&cx, &path, None);
    let text = text_of(&mut cx, &docs, &path);
    assert!(text_style(&text, "Strong_20_Emphasis").contains("fo:color=\"#ff0000\""));
    assert!(
        !text_style(&text, "Emphasis").contains("#ff0000"),
        "Emphasis stays"
    );
}

#[test]
fn quick_spacing_edits_keep_each_other() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Spacing");
    let props = panel(&mut cx, &docs);
    let spacing = |app: &gpui::App| props.read(app).state(super::paragraph::SPACING, app);
    settle(&cx, |app| spacing(app).is_some());
    let set = |cx: &mut TestAppContext, below: bool, points: f64| {
        let twips = (points * 20.0).round() as i32;
        let pair = (super::paragraph::SPACING, "paragraph.spacing");
        cx.update(|app| props.update(app, |props, cx| props.set_pair(pair, below, twips, cx)))
    };
    set(&mut cx, false, 6.0);
    set(&mut cx, true, 10.0);
    settle(&cx, |app| spacing(app).as_deref() == Some("212,353"));
    set(&mut cx, false, 8.0);
    set(&mut cx, true, 12.0);
    settle(&cx, |app| spacing(app).as_deref() == Some("282,423"));
    settled(&cx, &docs);
    let path = dir.path().join("spacing.docx");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, path.clone(), None, window, cx)
    });
    written(&cx, &path, None);
    let text = text_of(&mut cx, &docs, &path);
    assert!(
        text.contains("fo:margin-top=\"0.111in\" fo:margin-bottom=\"0.1665in\""),
        "both quick edits land"
    );
}

/// A window root that shows the panel.
struct Shown(Entity<Props>);

impl gpui::Render for Shown {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};
        gpui::div().size_full().child(self.0.clone())
    }
}

/// The top of the element marked `selector`.
fn top(cx: &mut gpui::VisualTestContext, selector: &'static str) -> Option<gpui::Pixels> {
    // Past reduced motion's 1ms folds.
    for _ in 0..3 {
        std::thread::sleep(std::time::Duration::from_millis(2));
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
    cx.debug_bounds(selector).map(|bounds| bounds.origin.y)
}

#[test]
fn in_a_table_the_table_group_comes_first_and_folds_stay_put() {
    const DOWN: i32 = 1024;
    const UP: i32 = 1025;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Table");
    let props = panel(&mut cx, &docs);
    let window = cx.add_window(|_, _| Shown(props.clone()));
    let mut shown = gpui::VisualTestContext::from_window(window.into(), &cx);
    assert_eq!(
        top(&mut shown, "doc-props-table"),
        None,
        "no table, no group"
    );
    let header = gpui::point(gpui::px(10.0), gpui::px(12.0));
    shown.simulate_click(header, gpui::Modifiers::none());
    assert_eq!(top(&mut shown, "doc-props-text"), None, "Text folds");
    shown.simulate_click(header, gpui::Modifiers::none());
    assert!(top(&mut shown, "doc-props-text").is_some(), "Text opens");
    with_window(&mut cx, &docs, |docs, _, cx| {
        docs.insert_table(3.0, 2.0, cx)
    });
    let in_table = |app: &gpui::App| props.read(app).state(".uno:DeleteTable", app);
    settle(&cx, |app| in_table(app).as_deref() == Some("enabled"));
    let table = top(&mut shown, "doc-props-table").expect("the table group");
    let text = top(&mut shown, "doc-props-text").expect("the text group");
    assert!(table < text, "the table group leads: {table:?} vs {text:?}");
    keys(&mut cx, &docs, &[UP]);
    settle(&cx, |app| in_table(app).as_deref() != Some("enabled"));
    shown.simulate_click(header, gpui::Modifiers::none());
    assert_eq!(top(&mut shown, "doc-props-text"), None, "Text folds again");
    keys(&mut cx, &docs, &[DOWN]);
    settle(&cx, |app| in_table(app).as_deref() == Some("enabled"));
    assert!(top(&mut shown, "doc-props-table").is_some(), "Table opens");
    assert_eq!(top(&mut shown, "doc-props-text"), None, "Text stays folded");
    keys(&mut cx, &docs, &[UP]);
    settle(&cx, |app| in_table(app).as_deref() != Some("enabled"));
    assert_eq!(
        top(&mut shown, "doc-props-text"),
        None,
        "and after the table"
    );
}
