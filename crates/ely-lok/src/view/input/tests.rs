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
    let worker = Mutex::new(worker);
    // Nothing merged to split: the kernel refuses.
    let input = Input::Run { commands: vec![("cells.split", Vec::new())], at: None };
    let (outcome, now) = run(&worker, &doc, input);
    assert!(outcome.is_err(), "the split fails");
    let reached = worker.lock().expect("the worker lock").now(&doc).expect("the state");
    assert_eq!(now, Some(reached.rev), "the view learns the revision anyway");
    assert!(reached.rev > doc.rev, "the request counted");
}
