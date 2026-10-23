//! Annotations and forms from the app (C11.4).

use ely_pdf_engine::{FieldKind, Marks, sample::{Extras, pdf_with}};
use gpui::{Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext};

use super::input::over;
use super::{frames, settle, shown, windowed};
use crate::annotate::Pick;
use crate::content::{Aim, Tool};
use crate::marking::Change;
use crate::view::PdfView;
use crate::viewer::Viewer;

/// Two lines and a text field, actual size.
fn marking<'a>(cx: &'a mut TestAppContext, dir: &std::path::Path) -> (Entity<Viewer>, &'a mut VisualTestContext, Entity<PdfView>) {
    let path = dir.join("doc.pdf");
    let extras = Extras { fields: &[0], ..Extras::default() };
    std::fs::write(&path, pdf_with(&[(612.0, 792.0, &["Hello world", "Second line"])], &[], &extras)).expect("writes");
    let (viewer, cx) = windowed(cx, dir, &[path], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > gpui::px(100.0));
    cx.update(|_, app| view.update(app, |view, cx| {
        view.zoom = crate::layout::Zoom::Scale(1.0);
        cx.notify();
    }));
    settle(cx, |app| view.read(app).notes.contains_key(&0));
    (viewer, cx, view)
}

fn tool(cx: &mut VisualTestContext, view: &Entity<PdfView>, tool: Tool) {
    cx.update(|_, app| view.update(app, |view, cx| view.set_tool(tool, cx)));
}

/// Presses at `from`, moves through `through`, lets go.
fn drag(cx: &mut VisualTestContext, view: &Entity<PdfView>, from: (f32, f32), through: &[(f32, f32)]) {
    // A hand presses what the last frame shows.
    frames(cx);
    let start = over(cx, view, 0, from);
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    let mut end = start;
    for at in through {
        end = over(cx, view, 0, *at);
        cx.simulate_mouse_move(end, MouseButton::Left, Modifiers::none());
    }
    cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
}

fn click(cx: &mut VisualTestContext, view: &Entity<PdfView>, at: (f32, f32)) {
    drag(cx, view, at, &[]);
}

/// Page one's notes once every edit has landed.
fn notes(cx: &mut VisualTestContext, view: &Entity<PdfView>) -> Marks {
    settle(cx, |app| view.read(app).settled() && view.read(app).notes.contains_key(&0));
    cx.update(|_, app| view.read(app).notes[&0].clone())
}

/// What the panel shows now, sent `change`.
fn send(cx: &mut VisualTestContext, viewer: &Entity<Viewer>, change: Change) {
    frames(cx);
    cx.update(|_, app| viewer.update(app, |viewer, cx| {
        let shown = viewer.note_panel.read(cx).shown.expect("the panel follows the pick");
        viewer.change_note(shown, change, cx)
    }));
}

fn kinds(marks: &Marks) -> Vec<String> {
    marks.annots.iter().map(|a| a.kind.clone()).collect()
}

#[test]
fn picked_text_highlights_in_one_step() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (_, cx, view) = marking(&mut cx, dir.path());
    tool(cx, &view, Tool::Highlight);
    drag(cx, &view, (74.0, 66.0), &[(140.0, 66.0)]);
    let marks = notes(cx, &view);
    assert_eq!(kinds(&marks), ["Highlight"]);
    assert!(!marks.annots[0].author.is_empty(), "signed by the account");
    assert!(cx.update(|_, app| view.read(app).selection.is_none()), "the pick is spent");
    cx.update(|_, app| view.update(app, |view, cx| view.undo(false, cx)));
    assert!(notes(cx, &view).annots.is_empty());
}

#[test]
fn shapes_and_ink_land_where_they_were_drawn() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (_, cx, view) = marking(&mut cx, dir.path());
    tool(cx, &view, Tool::Square);
    drag(cx, &view, (100.0, 400.0), &[(150.0, 420.0), (200.0, 450.0)]);
    let square = notes(cx, &view).annots[0].rect;
    assert!((square[0] - 100.0).abs() < 3.0 && (square[3] - 450.0).abs() < 3.0, "{square:?}");
    tool(cx, &view, Tool::Ink);
    drag(cx, &view, (300.0, 500.0), &[(320.0, 520.0), (340.0, 500.0), (360.0, 530.0)]);
    tool(cx, &view, Tool::Circle);
    click(cx, &view, (400.0, 400.0));
    assert_eq!(kinds(&notes(cx, &view)), ["Square", "Ink"], "a click draws no ellipse");
}

#[test]
fn notes_and_stamps_ask_first_then_change_and_go() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = marking(&mut cx, dir.path());
    tool(cx, &view, Tool::Note);
    click(cx, &view, (300.0, 300.0));
    assert!(matches!(cx.update(|_, app| view.read(app).aim), Some(Aim::Note(Pick::NewNote { .. }))));
    send(cx, &viewer, Change::Note("Check this".into()));
    notes(cx, &view);
    tool(cx, &view, Tool::Stamp);
    click(cx, &view, (300.0, 600.0));
    send(cx, &viewer, Change::Stamp("Draft".into()));
    let marks = notes(cx, &view);
    assert_eq!((kinds(&marks), marks.annots[0].contents.as_str(), marks.annots[1].icon.as_str()), (vec!["Text".into(), "Stamp".into()], "Check this", "Draft"));
    tool(cx, &view, Tool::Read);
    click(cx, &view, (305.0, 305.0));
    assert!(matches!(cx.update(|_, app| view.read(app).aim), Some(Aim::Note(Pick::Annot { .. }))), "a click picks the note");
    send(cx, &viewer, Change::Color(0x0000ff));
    let marks = notes(cx, &view);
    assert_eq!((marks.annots[0].rgb, marks.annots[0].contents.as_str()), (Some(0x0000ff), "Check this"));
    click(cx, &view, (305.0, 305.0));
    send(cx, &viewer, Change::Delete);
    assert_eq!(kinds(&notes(cx, &view)), ["Stamp"]);
}

/// Draws a field of `kind` over `at`, named `name`.
fn add_field(cx: &mut VisualTestContext, viewer: &Entity<Viewer>, view: &Entity<PdfView>, kind: FieldKind, at: [(f32, f32); 2], name: &str, options: &[&str]) {
    tool(cx, view, Tool::Field(kind));
    drag(cx, view, at[0], &[at[1]]);
    assert!(matches!(cx.update(|_, app| view.read(app).aim), Some(Aim::Note(Pick::NewField { .. }))));
    send(cx, viewer, Change::Design(name.into(), options.iter().map(|o| o.to_string()).collect()));
    notes(cx, view);
}

#[test]
fn drawn_fields_fill_from_the_page() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = marking(&mut cx, dir.path());
    add_field(cx, &viewer, &view, FieldKind::Checkbox, [(72.0, 300.0), (72.0, 300.0)], "Agree", &[]);
    add_field(cx, &viewer, &view, FieldKind::Choice, [(72.0, 340.0), (200.0, 360.0)], "Color", &["Red", "Blue"]);
    tool(cx, &view, Tool::Read);
    click(cx, &view, (78.0, 306.0));
    assert!(notes(cx, &view).fields[1].checked, "a click checks");
    click(cx, &view, (78.0, 306.0));
    assert!(!notes(cx, &view).fields[1].checked, "and clears");
    click(cx, &view, (100.0, 350.0));
    assert!(matches!(cx.update(|_, app| view.read(app).aim), Some(Aim::Note(Pick::Field { .. }))));
    send(cx, &viewer, Change::Fill("Blue".into()));
    notes(cx, &view);
    // The old field sits 652 points down.
    click(cx, &view, (100.0, 660.0));
    send(cx, &viewer, Change::Fill("Zoe".into()));
    let values: Vec<String> = notes(cx, &view).fields.iter().map(|f| f.value.clone()).collect();
    assert_eq!(values, ["Zoe", "Off", "Blue"]);
}

#[test]
fn a_second_radio_joins_the_group_and_one_stays_on() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = marking(&mut cx, dir.path());
    add_field(cx, &viewer, &view, FieldKind::Radio, [(72.0, 300.0), (72.0, 300.0)], "Size", &[]);
    tool(cx, &view, Tool::Field(FieldKind::Radio));
    click(cx, &view, (100.0, 300.0));
    frames(cx);
    let typed = cx.update(|_, app| viewer.read(app).note_panel.read(app).name.clone().expect("drawn").read(app).text().to_string());
    assert_eq!(typed, "Size", "suggests the page's group");
    send(cx, &viewer, Change::Design(typed, Vec::new()));
    notes(cx, &view);
    tool(cx, &view, Tool::Read);
    click(cx, &view, (78.0, 306.0));
    notes(cx, &view);
    click(cx, &view, (106.0, 306.0));
    let fields = notes(cx, &view).fields;
    assert_eq!((fields[1].checked, fields[2].checked), (false, true));
}

#[test]
fn a_click_on_a_box_while_an_edit_is_queued_does_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = marking(&mut cx, dir.path());
    add_field(cx, &viewer, &view, FieldKind::Checkbox, [(72.0, 300.0), (72.0, 300.0)], "Agree", &[]);
    tool(cx, &view, Tool::Read);
    cx.update(|_, app| view.update(app, |view, _| view.editing = true));
    click(cx, &view, (78.0, 306.0));
    assert!(cx.update(|_, app| view.read(app).queue.is_empty()), "nothing queued with old numbers");
}

#[test]
fn the_list_reads_every_page_and_jumps_to_a_pick() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = dir.path().join("three.pdf");
    let page: (f32, f32, &[&str]) = (612.0, 792.0, &["Hello"]);
    std::fs::write(&path, pdf_with(&[page, page, page], &[], &Extras::default())).expect("writes");
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > gpui::px(100.0));
    let note = ely_pdf_engine::NewAnnot { page: 2, shape: ely_pdf_engine::Shape::Note((100.0, 500.0)), rgb: None, contents: "Last".into(), author: String::new() };
    cx.update(|_, app| view.update(app, |view, cx| view.annotate("adding a note", vec![note], cx)));
    settle(cx, |app| view.read(app).settled());
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.side.update(cx, |side, cx| {
        side.pane = crate::side::Pane::Notes;
        cx.notify();
    })));
    settle(cx, |app| (0..3).all(|page| view.read(app).notes.contains_key(&page)));
    let id = cx.update(|_, app| view.read(app).notes[&2].annots[0].id);
    cx.update(|_, app| ely_ui::settings::panels::show(false, false, app));
    cx.update(|_, app| view.update(app, |view, cx| view.pick_note(2, id, cx)));
    frames(cx);
    assert_eq!(cx.update(|_, app| (view.read(app).current, view.read(app).aim)), (2, Some(Aim::Note(Pick::Annot { page: 2, id }))));
    assert!(cx.update(|_, app| ely_ui::settings::panels::shown(false, app)), "its editor opens");
}

#[test]
fn a_field_takes_a_new_name_and_options_and_no_change_is_no_step() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = marking(&mut cx, dir.path());
    add_field(cx, &viewer, &view, FieldKind::Choice, [(72.0, 340.0), (200.0, 360.0)], "Color", &["Red", "Blue"]);
    let edited = cx.update(|_, app| view.read(app).edited);
    cx.update(|_, app| ely_ui::settings::panels::show(false, false, app));
    click(cx, &view, (100.0, 350.0));
    assert!(matches!(cx.update(|_, app| view.read(app).aim), Some(Aim::Note(Pick::Field { .. }))), "the form tool picks a field");
    assert!(cx.update(|_, app| ely_ui::settings::panels::shown(false, app)), "its editor opens");
    send(cx, &viewer, Change::Design("Color".into(), vec!["Red".into(), "Blue".into()]));
    frames(cx);
    assert_eq!(cx.update(|_, app| view.read(app).edited), edited, "as it was");
    send(cx, &viewer, Change::Design("Shade".into(), vec!["Green".into()]));
    let field = notes(cx, &view).fields[1].clone();
    assert_eq!((field.name.as_str(), field.options), ("Shade", vec!["Green".to_string()]));
}

#[test]
fn a_highlight_before_the_text_is_read_lands_when_it_arrives() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (_, cx, view) = marking(&mut cx, dir.path());
    tool(cx, &view, Tool::Highlight);
    // A read seems in flight, so none starts yet.
    cx.update(|_, app| view.update(app, |view, _| {
        view.text.clear();
        view.asking_text.insert(0);
    }));
    drag(cx, &view, (74.0, 66.0), &[(140.0, 66.0)]);
    assert!(cx.update(|_, app| view.read(app).marking.is_some()), "waits for the text");
    assert!(notes(cx, &view).annots.is_empty());
    cx.update(|_, app| view.update(app, |view, cx| {
        view.asking_text.clear();
        view.page_text(0, cx);
    }));
    settle(cx, |app| view.read(app).edited == 1);
    assert_eq!(kinds(&notes(cx, &view)), ["Highlight"]);
}

#[test]
fn a_new_field_on_another_page_gets_a_free_name() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = dir.path().join("two.pdf");
    let page: (f32, f32, &[&str]) = (612.0, 792.0, &["Hello"]);
    std::fs::write(&path, pdf_with(&[page, page], &[], &Extras::default())).expect("writes");
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > gpui::px(100.0));
    let first = ely_pdf_engine::NewField { kind: FieldKind::Text, name: "Text1".into(), rect: [72.0, 300.0, 272.0, 320.0], options: Vec::new() };
    cx.update(|_, app| view.update(app, |view, cx| view.edit("adding a field", move |w, doc| w.add_field(doc, 0, &first), cx)));
    settle(cx, |app| view.read(app).settled());
    cx.update(|_, app| view.update(app, |view, cx| {
        view.tool = Tool::Field(FieldKind::Text);
        view.aim = Some(Aim::Note(Pick::NewField { page: 1, rect: [72.0, 300.0, 272.0, 320.0], kind: FieldKind::Text }));
        cx.notify();
    }));
    settle(cx, |app| viewer.read(app).note_panel.read(app).shown.is_some());
    let typed = cx.update(|_, app| viewer.read(app).note_panel.read(app).name.clone().expect("drawn").read(app).text().to_string());
    assert_eq!(typed, "Text2", "page one holds Text1");
}

#[test]
fn a_default_name_avoids_a_named_parent() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = dir.path().join("nested.pdf");
    let extras = Extras { fields: &[0], parent: Some("Text1"), ..Extras::default() };
    std::fs::write(&path, pdf_with(&[(612.0, 792.0, &["Hello"])], &[], &extras)).expect("writes");
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    cx.update(|_, app| view.update(app, |view, cx| {
        view.tool = Tool::Field(FieldKind::Text);
        view.aim = Some(Aim::Note(Pick::NewField { page: 0, rect: [72.0, 300.0, 272.0, 320.0], kind: FieldKind::Text }));
        cx.notify();
    }));
    settle(cx, |app| viewer.read(app).note_panel.read(app).shown.is_some());
    let typed = cx.update(|_, app| viewer.read(app).note_panel.read(app).name.clone().expect("drawn").read(app).text().to_string());
    assert_eq!(typed, "Text2", "Text1 holds Text1.field0");
}

#[test]
fn a_default_name_avoids_a_field_with_no_widget() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = dir.path().join("bare.pdf");
    let extras = Extras { bare: Some(("Text1", "Child")), ..Extras::default() };
    std::fs::write(&path, pdf_with(&[(612.0, 792.0, &["Hello"])], &[], &extras)).expect("writes");
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    cx.update(|_, app| view.update(app, |view, cx| {
        view.tool = Tool::Field(FieldKind::Text);
        view.aim = Some(Aim::Note(Pick::NewField { page: 0, rect: [72.0, 300.0, 272.0, 320.0], kind: FieldKind::Text }));
        cx.notify();
    }));
    settle(cx, |app| viewer.read(app).note_panel.read(app).shown.is_some());
    let typed = cx.update(|_, app| viewer.read(app).note_panel.read(app).name.clone().expect("drawn").read(app).text().to_string());
    assert_eq!(typed, "Text2", "Text1 has no widget but holds the name");
    send(cx, &viewer, Change::Design(typed, Vec::new()));
    assert_eq!(notes(cx, &view).fields.len(), 1, "the field lands");
}

#[test]
fn quick_strokes_queue_behind_each_other() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = dir.path().join("one.pdf");
    let page: (f32, f32, &[&str]) = (612.0, 792.0, &["Hello"]);
    std::fs::write(&path, pdf_with(&[page], &[], &Extras::default())).expect("writes");
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    let note = |y: f32| ely_pdf_engine::NewAnnot { page: 0, shape: ely_pdf_engine::Shape::Note((100.0, y)), rgb: None, contents: "n".into(), author: String::new() };
    // The second lands while the first still runs.
    cx.update(|_, app| view.update(app, |view, cx| {
        view.annotate("a first note", vec![note(500.0)], cx);
        view.annotate("a second note", vec![note(400.0)], cx);
    }));
    settle(cx, |app| view.read(app).settled());
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.side.update(cx, |side, cx| {
        side.pane = crate::side::Pane::Notes;
        cx.notify();
    })));
    let count = |cx: &mut gpui::VisualTestContext| {
        settle(cx, |app| view.read(app).settled() && view.read(app).notes.contains_key(&0));
        cx.update(|_, app| view.read(app).notes[&0].annots.len())
    };
    assert_eq!(count(cx), 2, "both notes landed");
    let rotate = |view: &mut PdfView, cx: &mut gpui::Context<PdfView>| {
        view.edit_pages("rotating", |worker, doc| worker.rotate_pages(doc, &[0], 90), cx)
    };
    // A running rotation would move the point: refused.
    cx.update(|_, app| view.update(app, |view, cx| {
        rotate(view, cx);
        view.annotate("a note while turning", vec![note(300.0)], cx);
    }));
    assert_eq!(count(cx), 2, "refused while pages move");
    // A rotation waiting behind another edit: refused too.
    cx.update(|_, app| view.update(app, |view, cx| {
        view.annotate("a third note", vec![note(250.0)], cx);
        rotate(view, cx);
        view.annotate("a note behind a turn", vec![note(200.0)], cx);
    }));
    assert_eq!(count(cx), 3, "only the note before the turn");
    cx.update(|_, app| view.update(app, |view, cx| view.annotate("a settled note", vec![note(150.0)], cx)));
    assert_eq!(count(cx), 4, "taken once pages are still");
}
