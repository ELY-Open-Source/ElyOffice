use super::*;
use crate::view::clip::{clip_of, flavor_of};
use ely_platform::clipboard::Clip;

fn stroke(text: &'static str) -> Keystroke {
    Keystroke::parse(text).expect("a keystroke")
}

#[test]
fn named_keys_map_to_vcl_codes() {
    assert_eq!(vcl_key(&stroke("enter")), Some((0, 1280)));
    assert_eq!(vcl_key(&stroke("shift-left")), Some((0, 1026 | SHIFT)));
    assert_eq!(vcl_key(&stroke("backspace")), Some((0, 1283)));
    assert_eq!(vcl_key(&stroke("f5")), Some((0, 772)));
    assert_eq!(vcl_key(&stroke("f1")), None, "kernel help stays shut");
    let (bold, redo) = match cfg!(target_os = "macos") {
        true => ("cmd-b", "cmd-shift-z"),
        false => ("ctrl-b", "ctrl-shift-z"),
    };
    assert_eq!(vcl_key(&stroke(bold)), Some((0, 513 | MOD1)));
    assert_eq!(vcl_key(&stroke(redo)), Some((0, 537 | MOD1 | SHIFT)));
}

#[test]
fn printable_text_waits_for_the_ime() {
    let mut typed = stroke("a");
    typed.key_char = Some("a".into());
    assert_eq!(vcl_key(&typed), None);
    let mut space = stroke("space");
    space.key_char = Some(" ".into());
    assert_eq!(vcl_key(&space), None);
}

#[test]
fn copied_flavors_pick_the_richest_clip() {
    let flavor = |mime: &str, data: &str| (mime.to_string(), data.as_bytes().to_vec());
    let html = clip_of(vec![
        flavor("text/html", "<b>x</b>"),
        flavor("text/plain;charset=utf-8", "x"),
    ]);
    assert_eq!(
        html,
        Some(Clip::Html {
            html: "<b>x</b>".into(),
            text: "x".into()
        })
    );
    assert_eq!(
        clip_of(vec![flavor("image/png", "png")]),
        Some(Clip::Png(b"png".to_vec()))
    );
    assert_eq!(clip_of(Vec::new()), None);
    assert_eq!(
        flavor_of(Clip::Text("t".into())).0,
        "text/plain;charset=utf-8"
    );
}

#[test]
fn copy_marks_read_back() {
    use crate::view::clip::mark_of;
    assert_eq!(mark_of("<b>x</b><!--ely-copy:7-3-->"), Some("7-3"));
    assert_eq!(mark_of("<b>x</b>"), None);
    let wrapped = "<html><body><b>x</b><!--ely-copy:7-4--></body></html>";
    assert_eq!(mark_of(wrapped), Some("7-4"), "the clipboard may wrap it");
}

#[test]
fn a_failed_input_still_reports_the_revision_it_reached() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("book.fods");
    std::fs::write(&source, crate::client::tests::input::FODS).expect("the fixture writes");
    let doc = worker.load(&crate::file_url(&source)).expect("the book loads");
    // Nothing merged to split: the kernel refuses.
    let input = Input::Run { commands: vec![("cells.split", Vec::new())], at: None };
    let (outcome, now, picture) = run(&mut worker, &doc, input);
    assert!(outcome.is_err(), "the split fails");
    let reached = worker.now(&doc).expect("the state");
    assert_eq!(now, Some(reached.rev), "the view learns the revision anyway");
    assert!(reached.rev > doc.rev, "the request counted");
    assert_eq!(picture, Err("it failed"), "a failed edit is never timed");
}

/// A typed key knows its screen, painted or not.
#[test]
fn a_key_knows_its_own_picture() {
    let Some(install) = crate::client::tests::install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let area = (0, 0, 3840, 3840);
    let key = || Input::Text { text: "7".into(), ends: true };
    for (name, body) in [("note.fodt", crate::client::tests::FODT), ("book.fods", crate::client::tests::input::FODS)] {
        let source = dir.path().join(name);
        std::fs::write(&source, body).expect("the fixture writes");
        let doc = worker.load(&crate::file_url(&source)).expect("the document loads");
        doc.visible_area(&mut worker, area, (64, 3840)).expect("the area sets");
        crate::client::tests::click(&mut worker, &doc, 150, 150);
        // Nothing painted: the kernel drops every invalidation.
        let (outcome, _, picture) = run(&mut worker, &doc, key());
        outcome.expect("the key posts");
        let (_, now, blind) = worker.state(&doc).expect("the state");
        assert!(blind, "{name}: an unpainted screen is blind");
        let Ok(Some(shows)) = picture else { panic!("{name}: an unseen key never timed: {picture:?}") };
        assert!(shows <= now, "{name}: the first paint shows the key");
        for n in 0..10 {
            // As the view: the dirtied screen repaints after each edit.
            doc.paint_tile(&mut worker, area, (64, 64)).expect("a tile paints");
            let (_, before, blind) = worker.state(&doc).expect("the state");
            assert!(!blind, "{name} key {n}: a painted screen is seen");
            let (outcome, _, picture) = run(&mut worker, &doc, key());
            outcome.expect("the key posts");
            let after = worker.state(&doc).expect("the state").1;
            // Seen, its callbacks land inside its own request.
            assert!(after > before, "{name} key {n} dirtied tiles");
            assert_eq!(picture, Ok(Some(after)), "{name} key {n} saw its own callbacks");
        }
    }
}
