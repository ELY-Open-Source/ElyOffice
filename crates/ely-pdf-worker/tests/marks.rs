//! Annotations and form fields against the real worker (PDF-05, PDF-06).

use std::path::Path;

use ely_pdf_engine::{
    Doc, FieldKind, NewAnnot, NewField, Shape, Worker,
    sample::{Extras, pdf_with},
};

fn worker() -> Worker {
    Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).expect("the worker")
}

/// A Letter page of two lines and a text field.
fn opened(path: &Path) -> (Worker, Doc) {
    let extras = Extras { fields: &[0], ..Extras::default() };
    std::fs::write(path, pdf_with(&[(612.0, 792.0, &["Hello world", "Second line"])], &[], &extras)).expect("writes");
    let mut worker = worker();
    let doc = worker.open(path).expect("opens").doc;
    (worker, doc)
}

fn note(page: usize, shape: Shape) -> NewAnnot {
    NewAnnot { page, shape, rgb: None, contents: String::new(), author: "Ann".into() }
}

/// Whether page one's pixels `at` hold colour `bgra`.
fn has_color(worker: &mut Worker, doc: Doc, at: [i32; 4], bgra: [u8; 3]) -> bool {
    let pixels = worker.render(doc, 0, (1.0, 0), at).expect("draws").bgra;
    pixels.chunks_exact(4).any(|p| p[..3].iter().zip(bgra).all(|(a, b)| a.abs_diff(b) < 40))
}

#[test]
fn every_kind_lands_in_one_step_and_reads_back() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(&dir.path().join("doc.pdf"));
    let hello: Vec<_> = worker.text(doc, 0).expect("text").into_iter().take(5).collect();
    let line = [hello[0].rect[0], hello[0].rect[1], hello[4].rect[2], hello[4].rect[3]];
    let before = worker.history(doc).expect("a mark");
    let mut noted = note(0, Shape::Note((300.0, 300.0)));
    noted.contents = "Check this".into();
    let mut marked = note(0, Shape::Highlight(vec![line]));
    marked.rgb = Some(0x00ff00);
    let all = [
        marked,
        note(0, Shape::Underline(vec![line])),
        note(0, Shape::StrikeOut(vec![line])),
        noted,
        note(0, Shape::Square([100.0, 400.0, 200.0, 450.0])),
        note(0, Shape::Circle([250.0, 400.0, 350.0, 450.0])),
        note(0, Shape::Line([100.0, 500.0, 300.0, 520.0])),
        note(0, Shape::Ink(vec![vec![(100.0, 600.0), (150.0, 620.0), (200.0, 600.0)]])),
        note(0, Shape::Stamp([350.0, 550.0, 540.0, 600.0], "Approved".into())),
    ];
    let after = worker.annotate(doc, &all).expect("annotates");
    doc = worker.now(doc).expect("the state");
    let read = worker.marks(doc, 0).expect("marks").annots;
    let kinds: Vec<&str> = read.iter().map(|a| a.kind.as_str()).collect();
    assert_eq!(kinds, ["Highlight", "Underline", "StrikeOut", "Text", "Square", "Circle", "Line", "Ink", "Stamp"]);
    assert_eq!((read[0].rgb, read[3].contents.as_str(), read[3].author.as_str()), (Some(0x00ff00), "Check this", "Ann"));
    assert_eq!(read[8].icon, "Approved");
    assert!(has_color(&mut worker, doc, [line[0] as i32, line[1] as i32, 20, 10], [0, 255, 0]), "the highlight is drawn");
    assert_eq!(worker.undo(doc, false).expect("undoes"), before, "one step");
    doc = worker.now(doc).expect("the state");
    assert!(worker.marks(doc, 0).expect("marks").annots.is_empty());
    assert_eq!(worker.undo(doc, true).expect("redoes"), after);
    doc = worker.now(doc).expect("the state");
    let saved = dir.path().join("saved.pdf");
    worker.save(doc, &saved).expect("saves");
    let mut other = self::worker();
    let reopened = other.open(&saved).expect("opens").doc;
    assert_eq!(other.marks(reopened, 0).expect("marks").annots.len(), 9, "kept in the file");
}

#[test]
fn a_bad_annotation_changes_nothing_and_keeps_redo() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(&dir.path().join("doc.pdf"));
    worker.annotate(doc, &[note(0, Shape::Square([100.0, 100.0, 200.0, 200.0]))]).expect("annotates");
    doc = worker.now(doc).expect("the state");
    worker.undo(doc, false).expect("undoes");
    doc = worker.now(doc).expect("the state");
    let good = note(0, Shape::Circle([100.0, 100.0, 200.0, 200.0]));
    for bad in [
        note(0, Shape::Stamp([100.0, 100.0, 200.0, 200.0], "Hacked".into())),
        note(0, Shape::Highlight(Vec::new())),
        note(0, Shape::Ink(vec![vec![(1.0, 1.0)]])),
        note(3, Shape::Note((10.0, 10.0))),
    ] {
        assert!(worker.annotate(doc, &[good.clone(), bad.clone()]).is_err(), "{bad:?}");
    }
    assert!(worker.marks(doc, 0).expect("marks").annots.is_empty(), "the good one did not land either");
    worker.undo(doc, true).expect("redo survives");
    doc = worker.now(doc).expect("the state");
    assert_eq!(worker.marks(doc, 0).expect("marks").annots[0].kind, "Square");
}

#[test]
fn annotations_change_and_go_by_their_number() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(&dir.path().join("doc.pdf"));
    let two = [note(0, Shape::Note((50.0, 50.0))), note(0, Shape::Square([100.0, 100.0, 200.0, 200.0]))];
    worker.annotate(doc, &two).expect("annotates");
    doc = worker.now(doc).expect("the state");
    let read = worker.marks(doc, 0).expect("marks").annots;
    worker.change_annot(doc, 0, read[0].id, Some(0x0000ff), Some("Done")).expect("changes");
    doc = worker.now(doc).expect("the state");
    worker.delete_annot(doc, 0, read[1].id).expect("deletes");
    doc = worker.now(doc).expect("the state");
    let now = worker.marks(doc, 0).expect("marks").annots;
    assert_eq!(now.len(), 1);
    assert_eq!((now[0].id, now[0].rgb, now[0].contents.as_str()), (read[0].id, Some(0x0000ff), "Done"));
    assert!(worker.delete_annot(doc, 0, read[1].id).is_err(), "gone already");
    worker.undo(doc, false).expect("undoes the delete");
    doc = worker.now(doc).expect("the state");
    assert_eq!(worker.marks(doc, 0).expect("marks").annots[1].id, read[1].id, "the same object back");
}

fn field(kind: FieldKind, name: &str, rect: [f32; 4]) -> NewField {
    let options = match kind {
        FieldKind::Choice => vec!["Red".into(), "Blue".into()],
        _ => Vec::new(),
    };
    NewField { kind, name: name.into(), rect, options }
}

#[test]
fn new_fields_fill_and_keep_their_values() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(&dir.path().join("doc.pdf"));
    for new in [
        field(FieldKind::Text, "Name", [72.0, 300.0, 272.0, 320.0]),
        field(FieldKind::Checkbox, "Agree", [72.0, 340.0, 86.0, 354.0]),
        field(FieldKind::Radio, "Size", [72.0, 370.0, 86.0, 384.0]),
        field(FieldKind::Radio, "Size", [100.0, 370.0, 114.0, 384.0]),
        field(FieldKind::Choice, "Color", [72.0, 400.0, 200.0, 420.0]),
    ] {
        worker.add_field(doc, 0, &new).expect("adds");
        doc = worker.now(doc).expect("the state");
    }
    assert!(!has_color(&mut worker, doc, [76, 304, 190, 12], [0, 0, 0]), "an empty box");
    let fields = worker.marks(doc, 0).expect("marks").fields;
    let names: Vec<(&str, FieldKind)> = fields.iter().map(|f| (f.name.as_str(), f.kind)).collect();
    assert_eq!(names, [
        ("field0", FieldKind::Text),
        ("Name", FieldKind::Text),
        ("Agree", FieldKind::Checkbox),
        ("Size", FieldKind::Radio),
        ("Size", FieldKind::Radio),
        ("Color", FieldKind::Choice),
    ]);
    assert_ne!(fields[3].on, fields[4].on, "each radio has its own state");
    assert_eq!(fields[5].options, ["Red", "Blue"]);
    worker.fill(doc, 0, fields[0].id, "changed").expect("fills the old field");
    doc = worker.now(doc).expect("the state");
    worker.fill(doc, 0, fields[1].id, "Zoe").expect("fills");
    doc = worker.now(doc).expect("the state");
    worker.fill(doc, 0, fields[2].id, &fields[2].on).expect("checks");
    doc = worker.now(doc).expect("the state");
    worker.fill(doc, 0, fields[3].id, &fields[3].on).expect("picks");
    doc = worker.now(doc).expect("the state");
    worker.fill(doc, 0, fields[4].id, &fields[4].on).expect("picks the other");
    doc = worker.now(doc).expect("the state");
    worker.fill(doc, 0, fields[5].id, "Blue").expect("chooses");
    doc = worker.now(doc).expect("the state");
    assert!(worker.fill(doc, 0, fields[3].id, "Nonsense").is_err(), "a state the button lacks");
    let saved = dir.path().join("saved.pdf");
    worker.save(doc, &saved).expect("saves");
    let mut other = self::worker();
    let reopened = other.open(&saved).expect("opens").doc;
    let read = other.marks(reopened, 0).expect("marks").fields;
    let values: Vec<(&str, bool)> = read.iter().map(|f| (f.value.as_str(), f.checked)).collect();
    assert_eq!(values[..2], [("changed", false), ("Zoe", false)]);
    assert!(read[2].checked && !read[3].checked && read[4].checked, "one radio of the group");
    assert_eq!(read[5].value, "Blue");
    assert!(has_color(&mut other, reopened, [76, 304, 190, 12], [0, 0, 0]), "the typed name is drawn");
}

#[test]
fn field_names_stay_unique_and_groups_go_with_their_last_button() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(&dir.path().join("doc.pdf"));
    assert!(worker.add_field(doc, 0, &field(FieldKind::Text, "field0", [72.0, 300.0, 272.0, 320.0])).is_err(), "taken");
    assert!(worker.add_field(doc, 0, &field(FieldKind::Radio, "field0", [72.0, 300.0, 86.0, 314.0])).is_err(), "not a group");
    assert!(worker.add_field(doc, 0, &field(FieldKind::Text, "a.b", [72.0, 300.0, 272.0, 320.0])).is_err(), "a dotted name");
    let mut empty = field(FieldKind::Choice, "Pick", [72.0, 300.0, 272.0, 320.0]);
    empty.options.clear();
    assert!(worker.add_field(doc, 0, &empty).is_err(), "a choice of nothing");
    worker.add_field(doc, 0, &field(FieldKind::Radio, "Size", [72.0, 370.0, 86.0, 384.0])).expect("adds");
    doc = worker.now(doc).expect("the state");
    let fields = worker.marks(doc, 0).expect("marks").fields;
    assert!(worker.change_field(doc, 0, fields[1].id, Some("field0"), None).is_err(), "renamed onto another");
    worker.change_field(doc, 0, fields[0].id, Some("Title"), None).expect("renames");
    doc = worker.now(doc).expect("the state");
    assert!(worker.change_field(doc, 0, fields[0].id, None, Some(&["x".into()])).is_err(), "text has no options");
    worker.delete_field(doc, 0, fields[1].id).expect("deletes the only button");
    doc = worker.now(doc).expect("the state");
    worker.add_field(doc, 0, &field(FieldKind::Text, "Size", [72.0, 300.0, 272.0, 320.0])).expect("the group is gone");
    doc = worker.now(doc).expect("the state");
    let names: Vec<String> = worker.marks(doc, 0).expect("marks").fields.into_iter().map(|f| f.name).collect();
    assert_eq!(names, ["Title", "Size"]);
}

#[test]
fn a_choice_takes_new_options_and_drops_a_lost_value() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(&dir.path().join("doc.pdf"));
    worker.add_field(doc, 0, &field(FieldKind::Choice, "Color", [72.0, 400.0, 200.0, 420.0])).expect("adds");
    doc = worker.now(doc).expect("the state");
    let id = worker.marks(doc, 0).expect("marks").fields[1].id;
    worker.fill(doc, 0, id, "Red").expect("chooses");
    doc = worker.now(doc).expect("the state");
    worker.change_field(doc, 0, id, None, Some(&["Red".into(), "Green".into()])).expect("keeps red");
    doc = worker.now(doc).expect("the state");
    assert_eq!(worker.marks(doc, 0).expect("marks").fields[1].value, "Red");
    worker.change_field(doc, 0, id, None, Some(&["Green".into()])).expect("drops red");
    doc = worker.now(doc).expect("the state");
    let read = worker.marks(doc, 0).expect("marks").fields[1].clone();
    assert_eq!((read.value.as_str(), read.options), ("", vec!["Green".to_string()]));
}

#[test]
fn a_filled_value_stays_drawn_through_undo_and_redo() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(&dir.path().join("doc.pdf"));
    worker.add_field(doc, 0, &field(FieldKind::Text, "Name", [72.0, 300.0, 272.0, 320.0])).expect("adds");
    doc = worker.now(doc).expect("the state");
    let id = worker.marks(doc, 0).expect("marks").fields[1].id;
    worker.fill(doc, 0, id, "Zoe").expect("fills");
    doc = worker.now(doc).expect("the state");
    worker.undo(doc, false).expect("undoes");
    doc = worker.now(doc).expect("the state");
    worker.undo(doc, true).expect("redoes before a draw");
    doc = worker.now(doc).expect("the state");
    assert!(has_color(&mut worker, doc, [76, 304, 190, 12], [0, 0, 0]), "drawn");
    worker.undo(doc, false).expect("undoes");
    doc = worker.now(doc).expect("the state");
    assert!(!has_color(&mut worker, doc, [76, 304, 190, 12], [0, 0, 0]), "gone");
    worker.undo(doc, true).expect("redoes");
    doc = worker.now(doc).expect("the state");
    assert!(has_color(&mut worker, doc, [76, 304, 190, 12], [0, 0, 0]), "drawn again");
}

#[test]
fn a_nested_field_renames_within_its_parent() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("doc.pdf");
    let extras = Extras { fields: &[0, 0], parent: Some("Person"), ..Extras::default() };
    std::fs::write(&path, pdf_with(&[(612.0, 792.0, &["Hello"])], &[], &extras)).expect("writes");
    let mut worker = worker();
    let mut doc = worker.open(&path).expect("opens").doc;
    let fields = worker.marks(doc, 0).expect("marks").fields;
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["Person.field0", "Person.field1"]);
    assert!(worker.change_field(doc, 0, fields[0].id, Some("field1"), None).is_err(), "a sibling's name");
    worker.change_field(doc, 0, fields[0].id, Some("First"), None).expect("renames");
    doc = worker.now(doc).expect("the state");
    worker.add_field(doc, 0, &field(FieldKind::Text, "First", [72.0, 300.0, 272.0, 320.0])).expect("a top field of the same leaf");
    doc = worker.now(doc).expect("the state");
    let names: Vec<String> = worker.marks(doc, 0).expect("marks").fields.into_iter().map(|f| f.name).collect();
    assert_eq!(names, ["Person.First", "Person.field1", "First"]);
}

#[test]
fn every_field_name_is_listed_widget_or_not() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("doc.pdf");
    let extras = Extras { fields: &[0], bare: Some(("Text1", "Child")), ..Extras::default() };
    std::fs::write(&path, pdf_with(&[(612.0, 792.0, &["Hello"])], &[], &extras)).expect("writes");
    let mut worker = worker();
    let doc = worker.open(&path).expect("opens").doc;
    assert_eq!(worker.field_names(doc).expect("names"), ["field0", "Text1", "Text1.Child"]);
    assert!(worker.add_field(doc, 0, &field(FieldKind::Text, "Text1", [72.0, 300.0, 272.0, 320.0])).is_err(), "the bare parent holds it");
}
