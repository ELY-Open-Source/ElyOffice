//! The first-line field against the real control (#76).

use ely_office::panel::Panel as _;
use ely_office::prelude::*;
use gpui::{Entity, IntoElement, Render, TestAppContext, VisualTestContext, Window};

use super::Props;
use super::paragraph::FIRST_LINE;
use crate::docs::Docs;
use crate::tests::{docs, settle, settled, text_of, with_window, written};

/// A window root that shows the field alone.
struct Field(Entity<Props>);

impl Render for Field {
    fn render(&mut self, _: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        self.0.update(cx, |props, cx| props.first_line_field(cx))
    }
}

/// Two paragraphs; the first takes `indent`.
fn two(indent: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
 xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0"
 xmlns:loext="urn:org:documentfoundation:names:experimental:office:xmlns:loext:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:automatic-styles><style:style style:name="P" style:family="paragraph">
 <style:paragraph-properties {indent}/>
 </style:style></office:automatic-styles>
 <office:body><office:text><text:p text:style-name="P">中文首行</text:p><text:p>中文排版</text:p></office:text></office:body>
</office:document>"#
    )
}

/// The panel on a document whose first paragraph has `indent`.
fn opened(cx: &mut TestAppContext, dir: &std::path::Path, indent: &str) -> Option<(Entity<Docs>, Entity<Props>)> {
    let docs = docs(cx, dir)?;
    cx.update(ely_gpui_component::init_for_tests);
    let path = dir.join("indent.fodt");
    std::fs::write(&path, two(indent)).expect("writes");
    with_window(cx, &docs, |docs, _, cx| docs.open_file(path.clone(), cx));
    settle(cx, |app| docs.read(app).core.active().is_some());
    settled(cx, &docs);
    let props = super::tests::panel(cx, &docs);
    Some((docs, props))
}

/// Focuses the field, presses `key` if any, leaves it.
fn visit(shown: &mut VisualTestContext, key: Option<&str>) {
    shown.update(|window, cx| window.focus_next(cx));
    // Typed text goes to the handler a paint registers.
    shown.update(|window, cx| window.draw(cx).clear(cx));
    if let Some(key) = key {
        shown.simulate_keystrokes(key);
    }
    shown.update(|window, cx| window.blur(cx));
    // Focus events go out with the next paint.
    shown.update(|window, cx| window.draw(cx).clear(cx));
    shown.run_until_parked();
}

#[test]
fn leaving_the_field_untyped_keeps_the_indent_and_its_unit() {
    // The indent, its state, a key, the state after it.
    let cases = [
        (r#"fo:text-indent="1cm""#, "56700,twip", "3", "300,ic"),
        (r#"fo:text-indent="0cm" loext:text-indent="2em""#, "200,em", "3", "300,ic"),
        (r#"fo:text-indent="0cm" loext:text-indent="1.25ic""#, "125,ic", "backspace", "120,ic"),
        // Past what the field takes: shown as a hint, kept.
        (r#"fo:text-indent="0cm" loext:text-indent="25ic""#, "2500,ic", "3", "300,ic"),
    ];
    for (indent, held, key, after) in cases {
        let dir = tempfile::tempdir().expect("a temp dir");
        let mut cx = TestAppContext::single();
        let Some((_docs, props)) = opened(&mut cx, dir.path(), indent) else {
            return eprintln!("SKIP: no LOK build");
        };
        let state = |app: &gpui::App| props.read(app).state(FIRST_LINE, app);
        settle(&cx, |app| state(app).as_deref() == Some(held));
        let window = cx.add_window(|_, _| Field(props.clone()));
        let mut shown = VisualTestContext::from_window(window.into(), &cx);
        shown.update(|window, _| window.activate_window());
        visit(&mut shown, None);
        // The same visit with a key pressed does change it.
        visit(&mut shown, Some(key));
        settle(&cx, |app| state(app).as_deref() == Some(after));
        // One step back: the file's own indent.
        cx.update(|app| props.update(app, |props, cx| props.run("edit.undo", &[], cx)));
        settle(&cx, |app| state(app).as_deref() != Some(after));
        assert_eq!(cx.update(|app| state(app)).as_deref(), Some(held), "{indent}: the visit alone changed it too");
    }
}

#[test]
fn a_redo_out_of_a_mixed_selection_shows_the_indent_again() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((docs, props)) = opened(&mut cx, dir.path(), r#"fo:text-indent="0cm" loext:text-indent="2ic""#) else {
        return eprintln!("SKIP: no LOK build");
    };
    let run = |cx: &mut TestAppContext, id: &str, values: &[&str]| cx.update(|app| props.update(app, |props, cx| props.run(id, values, cx)));
    let state = |app: &gpui::App, uno: &str| props.read(app).state(uno, app);
    let indent = |app: &gpui::App| state(app, FIRST_LINE);
    settle(&cx, |app| indent(app).as_deref() == Some("200,ic"));
    run(&mut cx, "edit.select_all", &[]);
    settle(&cx, |app| indent(app).as_deref() == Some(ely_lok::MIXED));
    run(&mut cx, "paragraph.first_line", &["-150"]);
    settle(&cx, |app| indent(app).as_deref() == Some("-150,ic"));
    run(&mut cx, "edit.undo", &[]);
    settle(&cx, |app| indent(app).as_deref() == Some(ely_lok::MIXED));
    run(&mut cx, "edit.redo", &[]);
    // The value the kernel sent before the mixed state.
    settle(&cx, |app| indent(app).as_deref() == Some("-150,ic"));
    let id = cx.update(|app| docs.read(app).core.active().expect("a tab"));
    let saved = dir.path().join("redo.odt");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, saved.clone(), None, window, cx));
    written(&cx, &saved, None);
    let flat = text_of(&mut cx, &docs, &saved);
    assert!(flat.contains(r#"loext:text-indent="-1.5ic""#) && !flat.contains(r#"loext:text-indent="2ic""#), "the redo did not reach the file");
}
