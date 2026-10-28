use std::path::PathBuf;

use gpui::TestAppContext;

use super::{
    Keys, window,
    writing::{NOTE, composing, opened, seeded},
};

#[test]
fn a_mail_link_opens_a_message_filled_from_it() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), format!("{NOTE}\r\nx").as_bytes(), &[]);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    let link = PathBuf::from("mailto:a@b.c?subject=Hi%20there&cc=d@e.f&body=one%0D%0Atwo");
    cx.update(|_, app| mail.update(app, |mail, cx| mail.take_files(vec![link], cx)));
    let compose = opened(cx);
    cx.update(|_, app| {
        let compose = compose.read(app).unwrap();
        assert_eq!(compose.to, ["a@b.c"]);
        assert_eq!(compose.cc, ["d@e.f"]);
        assert_eq!(compose.subject.as_ref(), "Hi there");
        assert!(!compose.markdown, "a link's body is plain text");
        assert_eq!(compose.document.read(app).text(app), "one\ntwo");
    });
}

#[test]
fn a_broken_mail_link_opens_nothing() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), format!("{NOTE}\r\nx").as_bytes(), &[]);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    let link = PathBuf::from("mailto:a@b.c?subject=%zz");
    cx.update(|_, app| mail.update(app, |mail, cx| mail.take_files(vec![link], cx)));
    cx.run_until_parked();
    assert!(composing(cx).is_empty());
}
