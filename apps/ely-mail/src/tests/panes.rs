use std::time::{Duration, Instant};

use ely_gpui_component::mail::Mail as Row;
use ely_mail_engine::{AccountId, Fetched, Filter, Key, MailboxId, Role, Store};
use gpui::{
    Entity, ScrollDelta, ScrollEvent, SharedString, TestAppContext, VisualTestContext, point,
    px, size,
};

use super::{Keys, account, settle, window};
use crate::{
    boxes::{self, Open},
    mail::Mail,
};

fn message(subject: &str, date: &str) -> Vec<u8> {
    format!(
        "From: Ann <ann@example.com>\r\nSubject: {subject}\r\nDate: {date}\r\n\
         Content-Type: text/plain\r\n\r\nhi\r\n"
    )
    .into_bytes()
}

pub(super) fn put(
    store: &mut Store,
    mailbox: MailboxId,
    uid: u32,
    flags: &[&str],
    subject: &str,
    day: u8,
) {
    let key = Key::Imap {
        uidvalidity: 1,
        uid,
    };
    let bytes = message(subject, &format!("{day} Jan 2024 10:00:00 +0000"));
    store
        .put(mailbox, &key, flags, Fetched::Whole(&bytes))
        .unwrap();
}

/// Ann has an inbox and an archive, Bob an inbox.
pub(super) fn seeded(dir: &std::path::Path) -> (AccountId, MailboxId) {
    let mut store = Store::open(dir).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let bob = store.add_account(&account("bob@example.com")).unwrap();
    let [ann_inbox, archive] = store
        .set_folders(
            ann,
            &[
                ("INBOX", Some(Role::Inbox)),
                ("Archive", Some(Role::Archive)),
            ],
        )
        .unwrap()
        .try_into()
        .unwrap();
    let [bob_inbox] = store
        .set_folders(bob, &[("INBOX", Some(Role::Inbox))])
        .unwrap()
        .try_into()
        .unwrap();
    put(&mut store, ann_inbox, 1, &["\\Seen"], "old", 1);
    put(&mut store, ann_inbox, 2, &[], "new", 3);
    put(&mut store, archive, 3, &[], "kept", 2);
    put(&mut store, bob_inbox, 4, &["\\Flagged"], "bob", 2);
    (ann, archive)
}

/// The listed subjects, once a list is read.
fn listed(mail: &Entity<Mail>, cx: &mut VisualTestContext) -> Option<Vec<String>> {
    cx.update(|_, app| match &mail.read(app).listing.mails {
        Some(Ok(rows)) => Some(rows.iter().map(|row| row.subject.to_string()).collect()),
        Some(Err(error)) => panic!("the list failed: {error}"),
        None => None,
    })
}

pub(super) fn shows(mail: &Entity<Mail>, cx: &mut VisualTestContext, subjects: &[&str]) {
    let want: Vec<String> = subjects.iter().map(|each| each.to_string()).collect();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while listed(mail, cx).as_ref() != Some(&want) {
        assert!(
            std::time::Instant::now() < deadline,
            "listed {:?}, not {want:?}",
            listed(mail, cx)
        );
        cx.run_until_parked();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn key_of(mail: &Entity<Mail>, cx: &mut VisualTestContext, subject: &str) -> SharedString {
    cx.update(|_, app| {
        let mails = mail.read(app).listing.mails.clone();
        let rows = mails.unwrap().unwrap();
        rows.iter()
            .find(|row| row.subject == subject)
            .unwrap()
            .key
            .clone()
    })
}

#[test]
fn every_inbox_is_one_list_newest_first() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path());
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.update(|_, app| assert_eq!(mail.read(app).open, Open::Inboxes));
    shows(&mail, cx, &["new", "bob", "old"]);
    settle(cx, |app| mail.read(app).inboxes_unread() == 2);
}

#[test]
fn filters_narrow_the_list() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path());
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    shows(&mail, cx, &["new", "bob", "old"]);
    let unread = Filter {
        unread: true,
        ..Filter::default()
    };
    cx.update(|_, app| mail.update(app, |mail, cx| mail.set_filter(unread, cx)));
    shows(&mail, cx, &["new", "bob"]);
    let flagged = Filter {
        unread: true,
        flagged: true,
        ..Filter::default()
    };
    cx.update(|_, app| mail.update(app, |mail, cx| mail.set_filter(flagged, cx)));
    shows(&mail, cx, &["bob"]);
}

#[test]
fn a_folder_lists_its_own_mail() {
    let dir = tempfile::tempdir().unwrap();
    let (ann, archive) = seeded(dir.path());
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    shows(&mail, cx, &["new", "bob", "old"]);
    let open = Open::Mailbox(ann, archive);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(open, cx)));
    shows(&mail, cx, &["kept"]);
}

#[test]
fn one_pick_fills_the_reader() {
    let dir = tempfile::tempdir().unwrap();
    let (ann, archive) = seeded(dir.path());
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    shows(&mail, cx, &["new", "bob", "old"]);
    let (new, bob) = (key_of(&mail, cx, "new"), key_of(&mail, cx, "bob"));
    let chosen = |cx: &mut VisualTestContext| {
        cx.update(|_, app| {
            let chosen = mail.read(app).listing.chosen.as_ref();
            chosen.map(|row| row.subject.to_string())
        })
    };
    cx.update(|_, app| mail.update(app, |mail, cx| mail.select(std::slice::from_ref(&new), cx)));
    assert_eq!(chosen(cx).as_deref(), Some("new"));
    cx.update(|_, app| mail.update(app, |mail, cx| mail.select(&[new, bob], cx)));
    assert_eq!(chosen(cx), None);
    let open = Open::Mailbox(ann, archive);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(open, cx)));
    cx.update(|_, app| assert!(mail.read(app).listing.selected.is_empty()));
}

#[test]
fn a_rebuilt_mailbox_drops_the_pick() {
    let dir = tempfile::tempdir().unwrap();
    let (ann, archive) = seeded(dir.path());
    // The newest row, whose id SQLite would hand out again.
    let mut store = Store::open(dir.path()).unwrap();
    put(&mut store, archive, 9, &[], "picked", 4);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    let open = Open::Mailbox(ann, archive);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(open, cx)));
    shows(&mail, cx, &["picked", "kept"]);
    let picked = key_of(&mail, cx, "picked");
    cx.update(|_, app| mail.update(app, |mail, cx| mail.select(&[picked], cx)));
    store.open_mailbox(archive, 2).unwrap();
    let key = Key::Imap {
        uidvalidity: 2,
        uid: 3,
    };
    let bytes = message("other", "2 Jan 2024 10:00:00 +0000");
    store
        .put(archive, &key, &[], Fetched::Whole(&bytes))
        .unwrap();
    drop(store);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.read_mail(cx)));
    shows(&mail, cx, &["other"]);
    cx.update(|_, app| {
        let listing = &mail.read(app).listing;
        assert!(listing.selected.is_empty(), "kept {:?}", listing.selected);
        assert!(listing.chosen.is_none());
    });
}

#[test]
fn picking_every_one_of_many_messages_is_quick() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    shows(&mail, cx, &[]);
    let rows: Vec<Row> = (0..100_000)
        .map(|n| Row::undated(n.to_string(), "Ann", "hi"))
        .collect();
    let keys: Vec<SharedString> = rows.iter().map(|row| row.key.clone()).collect();
    let listed = boxes::index(rows);
    cx.update(|_, app| mail.update(app, |mail, _| mail.listing.show(Some(Ok(listed)))));
    let started = Instant::now();
    cx.update(|_, app| mail.update(app, |mail, cx| mail.select(&keys, cx)));
    cx.update(|_, app| mail.update(app, |mail, cx| mail.select(&keys, cx)));
    let took = started.elapsed();
    assert!(took < Duration::from_secs(1), "picking took {took:?}");
    cx.update(|_, app| assert_eq!(mail.read(app).listing.selected.len(), 100_000));
}

#[test]
fn the_side_scrolls_past_many_folders_to_the_next_account() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let bob = store.add_account(&account("bob@example.com")).unwrap();
    let names: Vec<String> = (0..40).map(|n| format!("Folder {n:02}")).collect();
    let folders: Vec<_> = names.iter().map(|name| (name.as_str(), None)).collect();
    store.set_folders(ann, &folders).unwrap();
    store
        .set_folders(bob, &[("INBOX", Some(Role::Inbox))])
        .unwrap();
    drop(store);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(Open::Account(bob), cx)));
    cx.simulate_resize(size(px(1100.), px(800.)));
    settle(cx, |app| {
        let syncs = &mail.read(app).syncs;
        syncs.get(&ann).is_some_and(|each| each.folders.len() == 40) && syncs.contains_key(&bob)
    });
    let selector = "item-selected-Account settings";
    let height = cx.update(|window, _| window.viewport_size().height);
    let before = cx.debug_bounds(selector).expect("Bob's settings row");
    assert!(before.bottom() > height, "the side fits the window");
    let at = point(px(100.), height / 2.0);
    for _ in 0..20 {
        cx.simulate_event(ScrollEvent {
            position: at,
            delta: ScrollDelta::Pixels(point(px(0.), px(-1000.))),
            ..Default::default()
        });
        cx.run_until_parked();
    }
    let after = cx.debug_bounds(selector).expect("Bob's settings row");
    assert!(
        after.bottom() <= height,
        "Bob's settings are out of reach: {after:?}"
    );
}

#[test]
fn the_divider_keeps_its_place_when_the_window_opens_again() {
    use gpui::{Modifiers, MouseButton};
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path());
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.simulate_resize(size(px(1100.), px(800.)));
    settle(cx, |_| true);
    let list = cx.debug_bounds("mail-list-pane").expect("the list");
    // The divider's grab area starts at the list's edge.
    let grip = point(list.right() + px(2.), list.center().y);
    cx.simulate_mouse_down(grip, MouseButton::Left, Modifiers::none());
    // Starts the drag, sets its anchor, then moves.
    for x in [10., 20., 170.] {
        cx.simulate_mouse_move(grip + point(px(x), px(0.)), MouseButton::Left, Modifiers::none());
    }
    cx.simulate_mouse_up(grip + point(px(170.), px(0.)), MouseButton::Left, Modifiers::none());
    // The second draws what the first asked for.
    settle(cx, |_| true);
    settle(cx, |_| true);
    let wide = cx.debug_bounds("mail-list-pane").expect("the list").size.width;
    assert!(wide > list.size.width + px(100.), "the drag widens the list: {list:?} to {wide:?}");
    // As the tray: a new window around the same mail.
    let again = cx.update(|_, app| app.open_window(gpui::WindowOptions::default(), |_, _| mail.clone())).unwrap();
    cx.update(|window, _| window.remove_window());
    let mut cx = VisualTestContext::from_window(again.into(), cx);
    cx.simulate_resize(size(px(1100.), px(800.)));
    settle(&mut cx, |_| true);
    settle(&mut cx, |_| true);
    let kept = cx.debug_bounds("mail-list-pane").expect("the list").size.width;
    assert!((kept - wide).abs() < px(2.), "the list keeps its width: {wide:?} then {kept:?}");
}
