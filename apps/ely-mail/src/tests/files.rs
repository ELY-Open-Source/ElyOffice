//! A message's files: save, open, download, hand over (MAIL-10).

use std::time::Duration;

use ely_mail_engine::MessageId;
use gpui::{Entity, TestAppContext, VisualTestContext};

use super::{Keys, OPENED, settle, window, writing::{NOTE, opened, seeded}};
use crate::mail::Mail;

const FILED: &str = "Content-Type: multipart/mixed; boundary=b\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nsee the plan\r\n\
--b\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment; filename=plan.pdf\r\n\r\n%PDF-1.7 plan\r\n--b--\r\n";

/// Shows message `id`, its body read.
fn shown(mail: &Entity<Mail>, id: MessageId, cx: &mut VisualTestContext) {
    let key = id.0.to_string().into();
    cx.update(|_, app| mail.update(app, |mail, cx| mail.select(&[key], cx)));
    settle(cx, |app| mail.read(app).reading.files(id).is_some_and(|files| !files.is_empty()));
}

#[test]
fn a_file_saves_where_asked() {
    let dir = tempfile::tempdir().unwrap();
    let id = seeded(dir.path(), format!("{NOTE}{FILED}").as_bytes(), &[]);
    let mut cx = TestAppContext::single();
    let (mail, vcx) = window(&mut cx, dir.path(), Keys::default());
    shown(&mail, id, vcx);
    let target = dir.path().join("kept.pdf");
    let key = id.0.to_string().into();
    vcx.update(|_, app| mail.update(app, |mail, cx| mail.file_button(&key, 0, cx)));
    vcx.run_until_parked();
    let chosen = target.clone();
    vcx.simulate_new_path_selection(move |_| Some(chosen));
    vcx.run_until_parked();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !target.exists() && std::time::Instant::now() < deadline {
        vcx.run_until_parked();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(std::fs::read(&target).unwrap(), b"%PDF-1.7 plan");
}

#[test]
fn a_file_opens_as_a_copy_beside_the_mail() {
    let dir = tempfile::tempdir().unwrap();
    let id = seeded(dir.path(), format!("{NOTE}{FILED}").as_bytes(), &[]);
    let mut cx = TestAppContext::single();
    let (mail, vcx) = window(&mut cx, dir.path(), Keys::default());
    shown(&mail, id, vcx);
    let key = id.0.to_string().into();
    let copies = dir.path().join("opened").join(id.0.to_string());
    let mine = |copies: &std::path::Path| {
        OPENED.lock().unwrap().iter().filter(|each| each.starts_with(copies)).cloned().collect::<Vec<_>>()
    };
    for _ in 0..2 {
        vcx.update(|_, app| mail.update(app, |mail, cx| mail.open_file(&key, 0, cx)));
    }
    settle(vcx, |_| mine(&copies).len() == 2);
    let opened = mine(&copies);
    assert_eq!(opened[0], copies.join("plan.pdf"));
    assert_ne!(opened[0], opened[1], "a second open never overwrites the first copy");
    assert!(opened.iter().all(|copy| std::fs::read(copy).unwrap() == b"%PDF-1.7 plan"));
}

#[test]
fn a_waiting_file_brings_its_message_down() {
    let dir = tempfile::tempdir().unwrap();
    let partial = format!("{NOTE}{}", FILED.replace("%PDF-1.7 plan\r\n", ""));
    let id = seeded(dir.path(), partial.as_bytes(), &["2".to_owned()]);
    let mut cx = TestAppContext::single();
    let (mail, vcx) = window(&mut cx, dir.path(), Keys::default());
    shown(&mail, id, vcx);
    assert_eq!(vcx.update(|_, app| mail.read(app).reading.files(id).unwrap()[0].1), None);
    let key = id.0.to_string().into();
    vcx.update(|_, app| mail.update(app, |mail, cx| mail.open_file(&key, 0, cx)));
    assert!(vcx.update(|_, app| mail.read(app).downloading.contains(&id)), "a press downloads");
    settle(vcx, |app| mail.read(app).downloading.is_empty());
    let copies = dir.path().join("opened");
    assert!(!copies.exists(), "nothing opens before the file is here");
}

#[test]
fn a_whole_message_attached_waits_and_downloads() {
    let dir = tempfile::tempdir().unwrap();
    // Partial sync kept only the root's header.
    let head = "Content-Type: multipart/mixed; boundary=b\r\nContent-Disposition: attachment; filename=pack\r\n\r\n";
    let id = seeded(dir.path(), format!("{NOTE}{head}").as_bytes(), &["TEXT".to_owned()]);
    let mut cx = TestAppContext::single();
    let (mail, vcx) = window(&mut cx, dir.path(), Keys::default());
    shown(&mail, id, vcx);
    assert_eq!(vcx.update(|_, app| mail.read(app).reading.files(id).unwrap()[0].1), None, "not here, not empty");
    let key = id.0.to_string().into();
    vcx.update(|_, app| mail.update(app, |mail, cx| mail.file_button(&key, 0, cx)));
    assert!(vcx.update(|_, app| mail.read(app).downloading.contains(&id)), "the button downloads");
    settle(vcx, |app| mail.read(app).downloading.is_empty());
}

#[test]
fn files_handed_over_open_a_draft_carrying_them() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), format!("{NOTE}\r\nx").as_bytes(), &[]);
    let file = dir.path().join("notes.txt");
    std::fs::write(&file, "hello").unwrap();
    let mut cx = TestAppContext::single();
    let (mail, vcx) = window(&mut cx, dir.path(), Keys::default());
    vcx.update(|_, app| mail.update(app, |mail, cx| mail.write_with(vec![file], cx)));
    let compose = opened(vcx);
    settle(vcx, |app| compose.read(app).is_ok_and(|compose| compose.files.len() == 1));
    let sending = vcx.update(|_, app| compose.read(app).unwrap().sending);
    assert!(!sending, "it waits for the writer");
}

#[test]
fn a_file_named_with_a_path_opens_inside_its_folder() {
    let dir = tempfile::tempdir().unwrap();
    let named = FILED.replace("filename=plan.pdf", "filename=\"../../escaped.pdf\"");
    let id = seeded(dir.path(), format!("{NOTE}{named}").as_bytes(), &[]);
    let mut cx = TestAppContext::single();
    let (mail, vcx) = window(&mut cx, dir.path(), Keys::default());
    shown(&mail, id, vcx);
    let key = id.0.to_string().into();
    let copies = dir.path().join("opened").join(id.0.to_string());
    vcx.update(|_, app| mail.update(app, |mail, cx| mail.open_file(&key, 0, cx)));
    let mine = || OPENED.lock().unwrap().iter().filter(|each| each.starts_with(&copies)).cloned().collect::<Vec<_>>();
    settle(vcx, |_| !mine().is_empty());
    assert_eq!(mine(), [copies.join(".._.._escaped.pdf")]);
    assert!(!dir.path().join("escaped.pdf").exists());
    assert!(!dir.path().join("opened").join("escaped.pdf").exists());
}

#[cfg(windows)]
#[test]
fn a_file_that_runs_code_is_marked_and_never_opened() {
    let dir = tempfile::tempdir().unwrap();
    let named = FILED.replace("filename=plan.pdf", "filename=run.exe");
    let id = seeded(dir.path(), format!("{NOTE}{named}").as_bytes(), &[]);
    let mut cx = TestAppContext::single();
    let (mail, vcx) = window(&mut cx, dir.path(), Keys::default());
    shown(&mail, id, vcx);
    let key = id.0.to_string().into();
    let copies = dir.path().join("opened").join(id.0.to_string());
    vcx.update(|_, app| mail.update(app, |mail, cx| mail.open_file(&key, 0, cx)));
    let copy = copies.join("run.exe");
    settle(vcx, |_| copy.exists());
    for _ in 0..20 {
        vcx.run_until_parked();
        std::thread::sleep(Duration::from_millis(20));
    }
    let opened = OPENED.lock().unwrap().iter().any(|each| each.starts_with(&copies));
    assert!(!opened, "a program from mail never runs on a press");
    let zone = std::fs::read_to_string(format!("{}:Zone.Identifier", copy.display())).unwrap();
    assert!(zone.contains("ZoneId=3"), "{zone}");
}
