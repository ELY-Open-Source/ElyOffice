//! Message files and mbox files, in and out (MAIL-13).

use std::time::{Duration, Instant};

use ely_mail_engine::{Role, Store};
use gpui::{Modifiers, TestAppContext, VisualTestContext, WindowHandle, px, size};

use super::{Keys, account, organize::press_confirm, settle, window, writing::composing};
use crate::{boxes::Open, eml::Eml};

const EML: &[u8] = b"From: Ann <ann@example.com>\r\nTo: bo@example.com\r\nSubject: plan\r\n\
    Content-Type: multipart/mixed; boundary=b\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nsee the plan\r\n\
    --b\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment; filename=plan.pdf\r\n\r\n%PDF-1.7\r\n--b--\r\n";

const MBOX: &[u8] = b"From ann@example.com Tue Jan  2 10:00:00 2024\n\
    From: ann@example.com\nSubject: old one\nDate: Tue, 2 Jan 2024 10:00:00 +0000\n\nhi\n\n\
    From bo@example.com Wed Jan  3 10:00:00 2024\n\
    From: bo@example.com\nSubject: old two\nDate: Wed, 3 Jan 2024 10:00:00 +0000\n\nbye\n";

fn waited(path: &std::path::Path, cx: &mut VisualTestContext) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !path.exists() && Instant::now() < deadline {
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn eml_windows(cx: &mut VisualTestContext) -> Vec<WindowHandle<Eml>> {
    cx.update(|_, app| app.windows().into_iter().filter_map(|each| each.downcast::<Eml>()).collect())
}

#[test]
fn a_message_file_opens_to_read_and_others_go_into_a_draft() {
    let dir = tempfile::tempdir().unwrap();
    Store::open(dir.path()).unwrap().add_account(&account("bo@example.com")).unwrap();
    let (eml, pdf) = (dir.path().join("Plan.EML"), dir.path().join("plan.pdf"));
    std::fs::write(&eml, EML).unwrap();
    std::fs::write(&pdf, b"%PDF").unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.update(|_, app| mail.update(app, |mail, cx| mail.take_files(vec![eml, pdf], cx)));
    settle(cx, |app| {
        let read = app.windows().into_iter().filter_map(|each| each.downcast::<Eml>()).next();
        read.is_some_and(|window| window.read(app).is_ok_and(|eml| eml.shown.is_some()))
    });
    let windows = eml_windows(cx);
    assert_eq!(windows.len(), 1);
    let (subject, files) = cx.update(|_, app| {
        let eml = windows[0].read(app).unwrap();
        let shown = eml.shown.as_ref().unwrap().as_ref().unwrap();
        (shown.subject.clone(), shown.files.iter().map(|file| file.name.clone().unwrap()).collect::<Vec<_>>())
    });
    assert_eq!((subject.as_deref(), files), (Some("plan"), vec!["plan.pdf".to_owned()]));
    settle(cx, |app| app.windows().into_iter().any(|each| each.downcast::<crate::compose::Compose>().is_some()));
    let drafts = composing(cx);
    let attached = cx.update(|_, app| drafts[0].read(app).unwrap().files.len());
    assert_eq!((drafts.len(), attached), (1, 1), "the PDF went into one draft");
    let target = dir.path().join("kept.pdf");
    windows[0].update(cx, |eml, _, cx| eml.save_file(0, cx)).unwrap();
    cx.run_until_parked();
    let chosen = target.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    waited(&target, cx);
    assert_eq!(std::fs::read(&target).unwrap(), b"%PDF-1.7");
}

#[test]
fn an_mbox_file_comes_in_and_goes_out_on_this_computer_only() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap();
    drop(store);
    let file = dir.path().join("Old mail.mbox");
    std::fs::write(&file, MBOX).unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.simulate_resize(size(px(1100.), px(1400.)));
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(Open::Account(ann), cx)));
    settle(cx, |app| mail.read(app).syncs.get(&ann).is_some_and(|each| !each.folders.is_empty()));
    let at = cx.debug_bounds("mail-mbox-import").expect("the import button is laid out");
    cx.simulate_click(at.center(), Modifiers::none());
    cx.run_until_parked();
    let picked = file.clone();
    cx.simulate_path_prompt_response(move |_| Some(vec![picked]));
    settle(cx, |app| mail.read(app).syncs[&ann].folders.iter().any(|folder| folder.local));
    let folder = cx.update(|_, app| mail.read(app).syncs[&ann].folders.iter().find(|each| each.local).cloned().unwrap());
    assert_eq!(folder.name, "Old mail");
    let messages = Store::open(dir.path()).unwrap().list(ely_mail_engine::Scope::Mailbox(folder.id), Default::default());
    assert_eq!(messages.unwrap().len(), 2);
    assert_eq!(Store::open(dir.path()).unwrap().changes_waiting(ann).unwrap(), 0, "no server work");
    let out = dir.path().join("out.mbox");
    cx.update(|_, app| mail.update(app, |mail, cx| mail.export_mbox(folder.id, "Old mail", cx)));
    cx.run_until_parked();
    let chosen = out.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    waited(&out, cx);
    let written = std::fs::read_to_string(&out).unwrap();
    assert_eq!(written.matches("\nSubject: old ").count(), 2, "{written}");
    let name = "Old mail".into();
    cx.update(|_, app| mail.update(app, |mail, cx| mail.ask_remove_imported(ann, folder.id, name, cx)));
    press_confirm(&mail, cx);
    settle(cx, |app| !mail.read(app).syncs[&ann].folders.iter().any(|each| each.local));
}

#[test]
fn file_names_are_cleaned_and_keep_their_extension() {
    use crate::migrate::file_name_of;
    let cases = [
        ("plan.pdf", "plan.pdf"),
        ("../../escaped.pdf", ".._.._escaped.pdf"),
        (r"C:\x\a.pdf", "C__x_a.pdf"),
        ("con.txt", "_con.txt"),
        ("COM3.pdf", "_COM3.pdf"),
        ("Company report.pdf", "Company report.pdf"),
        ("..", "file"),
        ("notes", "notes"),
    ];
    for (name, cleaned) in cases {
        assert_eq!(file_name_of(name, "file"), cleaned, "{name}");
    }
    let long = file_name_of(&format!("{}.docx", "x".repeat(200)), "file");
    assert!(long.ends_with(".docx") && long.len() <= 90, "{long}");
}
