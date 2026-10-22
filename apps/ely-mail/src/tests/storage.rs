//! Mail's rows on the settings storage page (§18.3).

use ely_mail_engine::{Fetched, Key, Role, Store};
use ely_ui::settings::AppStorage;
use gpui::TestAppContext;

use super::{Keys, account, settle, window};

const PLAN: &[u8] = b"From: cy@example.com\r\nSubject: plan\r\n\r\nthe plan\r\n";

#[test]
fn the_storage_rows_read_sizes_and_clean_up_downloads() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let [inbox] = store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap().try_into().unwrap();
    let id = store.put(inbox, &Key::Imap { uidvalidity: 1, uid: 1 }, &[], Fetched::Whole(PLAN)).unwrap();
    drop(store);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.update(|_, app| crate::storage::install(mail.downgrade(), app));
    cx.update(|_, app| (app.global::<AppStorage>().read.clone())(app));
    let originals = |app: &gpui::App| mail.read(app).cache.sizes.as_ref().and_then(|sizes| sizes.as_ref().ok()).map(|sizes| sizes.originals);
    settle(cx, |app| originals(app) == Some(PLAN.len() as u64));
    let dialog = |cx: &mut gpui::VisualTestContext| cx.update(|_, app| (app.global::<AppStorage>().dialog.clone())(app).is_some());
    assert!(!dialog(cx), "no question before the button");
    cx.update(|_, app| mail.update(app, |mail, _| mail.cache.asking = true));
    assert!(dialog(cx));
    cx.update(|_, app| mail.update(app, |mail, cx| mail.forget_downloads(cx)));
    settle(cx, |app| originals(app) == Some(0));
    assert_eq!(Store::open(dir.path()).unwrap().original(id).unwrap(), None);
}

#[test]
fn all_mail_exports_once_a_file_per_folder() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let folders = [("INBOX", Some(Role::Inbox)), ("a/b", None), ("a_b", None), ("&g0l6P3ux-", None)];
    let inbox = store.set_folders(ann, &folders).unwrap()[0];
    store.put(inbox, &Key::Imap { uidvalidity: 1, uid: 1 }, &[], Fetched::Whole(PLAN)).unwrap();
    drop(store);
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    let account = out.join("ann@example.com");
    cx.update(|_, app| mail.update(app, |mail, cx| mail.export_all(out.clone(), cx)));
    settle(cx, |_| account.join("INBOX.mbox").exists());
    let mut names: Vec<_> = std::fs::read_dir(&account).unwrap().map(|each| each.unwrap().file_name()).collect();
    names.sort();
    assert_eq!(names, ["INBOX.mbox", "a_b 2.mbox", "a_b.mbox", "草稿箱.mbox"]);
    let written = std::fs::read(account.join("INBOX.mbox")).unwrap();
    assert!(written.starts_with(b"From cy@example.com "));
    // Again: everything is there, so nothing is written.
    std::fs::write(account.join("INBOX.mbox"), b"kept").unwrap();
    cx.update(|_, app| mail.update(app, |mail, cx| mail.export_all(out.clone(), cx)));
    for _ in 0..10 {
        cx.run_until_parked();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(std::fs::read(account.join("INBOX.mbox")).unwrap(), b"kept");
}

#[test]
fn accounts_that_clean_to_one_name_export_apart() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    for (address, uid) in [("a/b@example.com", 1), ("a_b@example.com", 2)] {
        let id = store.add_account(&account(address)).unwrap();
        let [inbox] = store.set_folders(id, &[("INBOX", Some(Role::Inbox))]).unwrap().try_into().unwrap();
        store.put(inbox, &Key::Imap { uidvalidity: 1, uid }, &[], Fetched::Whole(PLAN)).unwrap();
    }
    drop(store);
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.update(|_, app| mail.update(app, |mail, cx| mail.export_all(out.clone(), cx)));
    let second = out.join("a_b@example.com 2").join("INBOX.mbox");
    settle(cx, |_| second.exists());
    assert!(out.join("a_b@example.com").join("INBOX.mbox").exists());
}

#[test]
fn the_download_cap_is_kept_and_trimmed_mail_offers_its_body() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let [inbox] = store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap().try_into().unwrap();
    let id = store.put(inbox, &Key::Imap { uidvalidity: 1, uid: 1 }, &[], Fetched::Whole(PLAN)).unwrap();
    drop(store);
    let mut cx = TestAppContext::single();
    let (mail, vcx) = window(&mut cx, dir.path(), Keys::default());
    settle(vcx, |app| mail.read(app).cap.gb == Some(Ok(None)));
    vcx.update(|_, app| mail.update(app, |mail, cx| mail.set_cap(Some(5), cx)));
    let saved = dir.path().join("settings.json");
    settle(vcx, |_| std::fs::read_to_string(&saved).is_ok_and(|text| text.contains("\"mail_cache\": 5")));
    let mut again = TestAppContext::single();
    let (reopened, rcx) = window(&mut again, dir.path(), Keys::default());
    settle(rcx, |app| reopened.read(app).cap.gb == Some(Ok(Some(5))));
    vcx.update(|_, app| mail.update(app, |mail, cx| mail.trim_to(0, cx)));
    let key = id.0.to_string().into();
    vcx.update(|_, app| mail.update(app, |mail, cx| mail.select(&[key], cx)));
    let button: &'static str = format!("mail-download-{}", id.0).leak();
    settle(vcx, |_| true);
    let mut tries = 0;
    while vcx.debug_bounds(button).is_none() && tries < 100 {
        vcx.run_until_parked();
        std::thread::sleep(std::time::Duration::from_millis(20));
        tries += 1;
    }
    assert!(vcx.debug_bounds(button).is_some(), "a trimmed message can come down again");
}

#[test]
fn the_last_cap_chosen_is_the_one_kept_and_used() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let [inbox] = store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap().try_into().unwrap();
    let id = store.put(inbox, &Key::Imap { uidvalidity: 1, uid: 1 }, &[], Fetched::Whole(PLAN)).unwrap();
    drop(store);
    let mut cx = TestAppContext::single();
    let (mail, vcx) = window(&mut cx, dir.path(), Keys::default());
    settle(vcx, |app| mail.read(app).cap.gb == Some(Ok(None)));
    // A trim asked before a newer choice is stale.
    vcx.update(|_, app| {
        mail.update(app, |mail, cx| {
            mail.trim_to(0, cx);
            mail.set_cap(Some(1), cx);
            mail.set_cap(None, cx);
        })
    });
    let saved = dir.path().join("settings.json");
    settle(vcx, |_| std::fs::read_to_string(&saved).is_ok_and(|text| text.contains("\"mail_cache\": null")));
    for _ in 0..20 {
        vcx.run_until_parked();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let text = std::fs::read_to_string(&saved).unwrap();
    assert!(text.contains("\"mail_cache\": null"), "an older choice never lands last: {text}");
    let kept = vcx.update(|_, app| mail.read(app).store.clone().unwrap().lock().unwrap().original(id).unwrap());
    assert!(kept.is_some(), "no cap now, nothing trimmed");
}
