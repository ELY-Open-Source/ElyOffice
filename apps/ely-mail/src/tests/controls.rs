use std::{
    net::TcpListener,
    sync::mpsc,
    time::{Duration, Instant},
};

use ely_mail_engine::{AccountId, LocalFolder, MailboxId, Role, Store};
use gpui::{
    Entity, ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext, point, px, size,
};

use super::{Keys, account, settle, window};
use crate::{boxes::Open, mail::Mail, syncing::EVERY, view::imap_range};

/// An account on a server that never answers.
fn silent(dir: &std::path::Path, port: u16) -> AccountId {
    let mut ann = account("ann@example.com");
    ann.incoming.port = port;
    let mut store = Store::open(dir).unwrap();
    let id = store.add_account(&ann).unwrap();
    store
        .set_folders(id, &[("INBOX", Some(Role::Inbox))])
        .unwrap();
    id
}

fn with_secret() -> Keys {
    let keys = Keys::default();
    let secret = ("account:ann@example.com".to_owned(), "pw".to_owned());
    keys.kept.lock().unwrap().extend([secret]);
    keys
}

/// Waits on the network task alone, not the window.
fn finished(mail: &Entity<Mail>, id: AccountId, cx: &mut VisualTestContext) {
    let deadline = Instant::now() + Duration::from_secs(20);
    let done = |cx: &mut VisualTestContext| {
        cx.update(|_, app| {
            let task = mail.read(app).syncs[&id].task.as_ref();
            task.unwrap().is_finished()
        })
    };
    while !done(cx) {
        assert!(Instant::now() < deadline, "the sync never ended");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// An account whose sync fails at once: no secret saved.
fn failing(dir: &std::path::Path) -> AccountId {
    let mut store = Store::open(dir).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    store
        .set_folders(ann, &[("INBOX", Some(Role::Inbox))])
        .unwrap();
    ann
}

fn failed(mail: &Entity<Mail>, id: AccountId, cx: &mut VisualTestContext) -> bool {
    cx.update(|_, app| mail.read(app).syncs[&id].failed.is_some())
}

fn forget(mail: &Entity<Mail>, id: AccountId, cx: &mut VisualTestContext) {
    cx.update(|_, app| {
        mail.update(app, |mail, _| {
            mail.syncs.get_mut(&id).unwrap().failed = None
        })
    });
}

fn settled_failure(mail: &Entity<Mail>, id: AccountId, cx: &mut VisualTestContext) {
    settle(cx, |app| {
        mail.read(app)
            .syncs
            .get(&id)
            .is_some_and(|each| each.failed.is_some() && each.task.is_none())
    });
}

#[test]
fn every_account_syncs_again_on_the_timer() {
    let dir = tempfile::tempdir().unwrap();
    let ann = failing(dir.path());
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    settled_failure(&mail, ann, cx);
    forget(&mail, ann, cx);
    cx.executor().advance_clock(EVERY);
    settled_failure(&mail, ann, cx);
}

#[test]
fn a_paused_sync_waits_until_resumed() {
    let dir = tempfile::tempdir().unwrap();
    let ann = failing(dir.path());
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    settled_failure(&mail, ann, cx);
    forget(&mail, ann, cx);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.set_paused(true, cx)));
    cx.update(|_, app| mail.update(app, |mail, cx| mail.sync(ann, cx)));
    cx.executor().advance_clock(EVERY);
    cx.run_until_parked();
    assert!(!failed(&mail, ann, cx), "a paused account synced");
    cx.update(|_, app| mail.update(app, |mail, cx| mail.set_paused(false, cx)));
    settled_failure(&mail, ann, cx);
}

#[test]
fn a_folder_kept_offline_is_saved_and_synced() {
    let dir = tempfile::tempdir().unwrap();
    let ann = failing(dir.path());
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    settle(cx, |app| {
        mail.read(app)
            .syncs
            .get(&ann)
            .is_some_and(|each| each.failed.is_some() && !each.folders.is_empty())
    });
    let inbox = cx.update(|_, app| mail.read(app).syncs[&ann].folders[0].clone());
    assert!(!inbox.offline);
    forget(&mail, ann, cx);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.set_offline(ann, inbox.id, true, cx)));
    settle(cx, |app| {
        let syncing = &mail.read(app).syncs[&ann];
        syncing.folders[0].offline && syncing.failed.is_some() && syncing.task.is_none()
    });
    assert!(Store::open(dir.path()).unwrap().offline(inbox.id).unwrap());
}

#[test]
fn an_old_end_queued_across_a_resume_keeps_the_new_sync() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let ann = silent(dir.path(), listener.local_addr().unwrap().port());
    let keys = Keys::default();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), keys.clone());
    settled_failure(&mail, ann, cx);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.sync(ann, cx)));
    finished(&mail, ann, cx);
    keys.kept
        .lock()
        .unwrap()
        .insert("account:ann@example.com".into(), "pw".into());
    let resumed = cx.update(|_, app| {
        mail.update(app, |mail, cx| {
            mail.set_paused(true, cx);
            mail.set_paused(false, cx);
            mail.syncs[&ann].task.clone().unwrap()
        })
    });
    cx.run_until_parked();
    assert!(
        cx.update(|_, app| mail.read(app).syncs[&ann].task.is_some()),
        "an old end cleared the new sync"
    );
    cx.update(|_, app| mail.update(app, |mail, cx| mail.set_paused(true, cx)));
    let deadline = Instant::now() + Duration::from_secs(5);
    while !resumed.is_finished() {
        assert!(Instant::now() < deadline, "pausing left the sync running");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn an_old_end_after_a_failed_removal_keeps_the_new_sync() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let ann = silent(dir.path(), listener.local_addr().unwrap().port());
    let keys = Keys::default();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), keys.clone());
    settled_failure(&mail, ann, cx);
    cx.update(|_, app| {
        mail.update(app, |mail, cx| {
            mail.stop_sync(ann);
            mail.sync(ann, cx);
        })
    });
    finished(&mail, ann, cx);
    keys.kept
        .lock()
        .unwrap()
        .insert("account:ann@example.com".into(), "pw".into());
    let restarted = cx.update(|_, app| {
        mail.update(app, |mail, cx| {
            mail.stop_sync(ann);
            mail.sync(ann, cx);
            mail.syncs[&ann].task.clone().unwrap()
        })
    });
    cx.run_until_parked();
    assert!(
        cx.update(|_, app| mail.read(app).syncs[&ann].task.is_some()),
        "an old end cleared the restarted sync"
    );
    cx.update(|_, app| mail.update(app, |mail, cx| mail.set_paused(true, cx)));
    let deadline = Instant::now() + Duration::from_secs(5);
    while !restarted.is_finished() {
        assert!(Instant::now() < deadline, "pausing left the sync running");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn a_folder_kept_offline_mid_sync_syncs_once_more() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (hang_up, hung) = mpsc::channel::<()>();
    let server = std::thread::spawn(move || {
        let held = listener.accept().unwrap();
        hung.recv().unwrap();
        drop((held, listener));
    });
    let dir = tempfile::tempdir().unwrap();
    let ann = silent(dir.path(), port);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), with_secret());
    settle(cx, |app| {
        mail.read(app)
            .syncs
            .get(&ann)
            .is_some_and(|each| each.task.is_some() && !each.folders.is_empty())
    });
    let (inbox, run) = cx.update(|_, app| {
        let syncing = &mail.read(app).syncs[&ann];
        (syncing.folders[0].id, syncing.run)
    });
    cx.update(|_, app| mail.update(app, |mail, cx| mail.set_offline(ann, inbox, true, cx)));
    settle(cx, |app| mail.read(app).syncs[&ann].again);
    hang_up.send(()).unwrap();
    server.join().unwrap();
    settle(cx, |app| {
        let syncing = &mail.read(app).syncs[&ann];
        syncing.run == run + 1 && syncing.task.is_none() && syncing.failed.is_some()
    });
}

#[test]
fn the_account_page_scrolls_to_its_last_folder() {
    let dir = tempfile::tempdir().unwrap();
    let ann = failing(dir.path());
    let names: Vec<String> = (0..40).map(|n| format!("Folder {n:02}")).collect();
    let folders: Vec<_> = names.iter().map(|name| (name.as_str(), None)).collect();
    let last = *Store::open(dir.path())
        .unwrap()
        .set_folders(ann, &folders)
        .unwrap()
        .last()
        .unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(Open::Account(ann), cx)));
    cx.simulate_resize(size(px(1100.), px(800.)));
    settle(cx, |app| {
        mail.read(app)
            .syncs
            .get(&ann)
            .is_some_and(|each| each.folders.len() == 40)
    });
    let selector: &'static str = format!("mail-offline-{}", last.0).leak();
    let height = cx.update(|window, _| window.viewport_size().height);
    let before = cx
        .debug_bounds(selector)
        .expect("the last switch is laid out");
    assert!(before.bottom() > height, "the folders fit the window");
    let at = point(px(400.), height / 2.0);
    for _ in 0..20 {
        cx.simulate_event(ScrollWheelEvent {
            position: at,
            delta: ScrollDelta::Pixels(point(px(0.), px(-1000.))),
            ..Default::default()
        });
        cx.run_until_parked();
    }
    let after = cx
        .debug_bounds(selector)
        .expect("the last switch is laid out");
    assert!(
        after.bottom() <= height,
        "the last switch is out of reach: {after:?}"
    );
}

#[test]
fn the_range_names_the_folders_kept_whole() {
    let folder = |id, name: &str, offline| LocalFolder {
        id: MailboxId(id),
        name: name.into(),
        shown: name.into(),
        role: None,
        offline,
        pinned: false,
        local: false,
    };
    let cx = TestAppContext::single();
    cx.update(|app| {
        ely_ui::i18n::install_as("en-US", app);
        let rows = |folders: &[LocalFolder]| -> Vec<String> {
            imap_range(folders, app)
                .into_iter()
                .map(|(key, value)| format!("{key}: {value}"))
                .collect()
        };
        let both = [folder(1, "INBOX", true), folder(2, "Sent", false), folder(3, "Work", true)];
        assert_eq!(
            rows(&both),
            [
                "Downloaded: Other folders: headers of all mail; text from the last 90 days; attachments when asked",
                "Kept whole: Every message in INBOX, Work, with attachments",
            ]
        );
        assert_eq!(rows(&[folder(1, "INBOX", true)]), ["Kept whole: Every message in INBOX, with attachments"]);
        assert_eq!(
            rows(&[folder(2, "Sent", false)]),
            ["Downloaded: Headers of all mail; text from the last 90 days; attachments when asked"]
        );
        let drafts = LocalFolder {
            name: "&g0l6P3ux-".into(),
            shown: "草稿箱".into(),
            ..folder(4, "", true)
        };
        assert_eq!(rows(std::slice::from_ref(&drafts)), ["Kept whole: Every message in 草稿箱, with attachments"]);
        assert_eq!(crate::controls::offline_label(&drafts, app), "Keep 草稿箱 whole offline");
    });
}

#[test]
fn remove_needs_an_open_account() {
    let dir = tempfile::tempdir().unwrap();
    Store::open(dir.path())
        .unwrap()
        .add_account(&account("ann@example.com"))
        .unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    settle(cx, |app| mail.read(app).accounts().len() == 1);
    cx.update(|_, app| assert_eq!(mail.read(app).open, Open::Inboxes));
    cx.update(|_, app| mail.update(app, |mail, cx| mail.ask_remove(cx)));
    cx.run_until_parked();
    cx.update(|_, app| {
        let mail = mail.read(app);
        assert!(mail.removing.is_none());
        assert_eq!(mail.accounts().len(), 1);
    });
}
