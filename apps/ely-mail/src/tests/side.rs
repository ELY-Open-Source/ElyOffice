//! Accounts fold, folders pin to the top, and both stay.

use ely_mail_engine::{AccountId, MailboxId, Role, Store};
use gpui::{Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext, px, size};

use super::{Keys, account, settle, window};
use crate::{boxes::Open, mail::Mail};

/// Ann with an inbox and Sent, open in a window.
fn opened<'a>(
    dir: &std::path::Path,
    cx: &'a mut TestAppContext,
) -> (
    Entity<Mail>,
    &'a mut VisualTestContext,
    AccountId,
    [MailboxId; 2],
) {
    let mut store = Store::open(dir).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let folders = [("INBOX", Some(Role::Inbox)), ("Sent", Some(Role::Sent))];
    let boxes: [MailboxId; 2] = store
        .set_folders(ann, &folders)
        .unwrap()
        .try_into()
        .unwrap();
    drop(store);
    let (mail, cx) = window(cx, dir, Keys::default());
    cx.simulate_resize(size(px(1100.), px(800.)));
    settle(cx, |app| {
        let syncs = &mail.read(app).syncs;
        syncs.get(&ann).is_some_and(|each| each.folders.len() == 2)
    });
    (mail, cx, ann, boxes)
}

#[test]
fn a_folded_account_hides_its_folders_and_stays_folded() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx, ann, [inbox, _]) = opened(dir.path(), &mut cx);
    let open = Open::Mailbox(ann, inbox);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(open, cx)));
    settle(cx, |_| true);
    assert!(cx.debug_bounds("item-selected-Inbox").is_some());
    let title = cx.debug_bounds("fold ann@example.com").expect("a title");
    cx.simulate_click(title.center(), Modifiers::none());
    settle(cx, |app| mail.read(app).folded.contains(&ann));
    assert!(cx.debug_bounds("item-selected-Inbox").is_none());
    assert_eq!(Store::open(dir.path()).unwrap().folded().unwrap(), [ann]);
}

#[test]
fn a_pinned_folder_moves_above_its_account() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx, ann, [_, sent]) = opened(dir.path(), &mut cx);
    let open = Open::Mailbox(ann, sent);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(open, cx)));
    settle(cx, |_| true);
    let row = cx.debug_bounds("item-selected-Sent").expect("Sent");
    let title = cx.debug_bounds("fold ann@example.com").expect("a title");
    assert!(row.top() > title.top(), "{row:?} under {title:?}");
    cx.simulate_mouse_down(row.center(), MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(row.center(), MouseButton::Right, Modifiers::none());
    settle(cx, |_| true);
    let pin = cx.debug_bounds("menu-row-Pin to top").expect("the menu");
    cx.simulate_click(pin.center(), Modifiers::none());
    settle(cx, |app| {
        let folders = &mail.read(app).syncs[&ann].folders;
        folders
            .iter()
            .any(|folder| folder.id == sent && folder.pinned)
    });
    let row = cx.debug_bounds("item-selected-Sent").expect("Sent");
    let title = cx.debug_bounds("fold ann@example.com").expect("a title");
    assert!(row.top() < title.top(), "{row:?} over {title:?}");
    let held = Store::open(dir.path()).unwrap().folders(ann).unwrap();
    assert!(held.iter().any(|folder| folder.id == sent && folder.pinned));
}

#[test]
fn the_keyboard_pins_a_folder() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx, ann, [_, sent]) = opened(dir.path(), &mut cx);
    let open = Open::Mailbox(ann, sent);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(open, cx)));
    settle(cx, |_| true);
    let row = cx.debug_bounds("item-selected-Sent").expect("Sent");
    cx.simulate_click(row.center(), Modifiers::none());
    settle(cx, |_| true);
    cx.simulate_keystrokes("shift-f10");
    settle(cx, |_| true);
    let pin = cx.debug_bounds("menu-row-Pin to top").expect("the menu");
    cx.simulate_click(pin.center(), Modifiers::none());
    settle(cx, |app| {
        let folders = &mail.read(app).syncs[&ann].folders;
        folders.iter().any(|folder| folder.id == sent && folder.pinned)
    });
}

#[test]
fn escape_leaves_the_menu_key_working() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx, ann, [_, sent]) = opened(dir.path(), &mut cx);
    let open = Open::Mailbox(ann, sent);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(open, cx)));
    settle(cx, |_| true);
    let row = cx.debug_bounds("item-selected-Sent").expect("Sent");
    cx.simulate_click(row.center(), Modifiers::none());
    settle(cx, |_| true);
    cx.simulate_keystrokes("shift-f10");
    settle(cx, |_| true);
    assert!(cx.debug_bounds("menu-row-Pin to top").is_some());
    cx.simulate_keystrokes("escape");
    settle(cx, |_| true);
    assert!(cx.debug_bounds("menu-row-Pin to top").is_none());
    cx.simulate_keystrokes("shift-f10");
    settle(cx, |_| true);
    assert!(cx.debug_bounds("menu-row-Pin to top").is_some(), "no second menu");
}

#[test]
fn a_shut_menu_stays_shut_when_the_window_opens_again() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx, ann, [_, sent]) = opened(dir.path(), &mut cx);
    let open = Open::Mailbox(ann, sent);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(open, cx)));
    settle(cx, |_| true);
    let row = cx.debug_bounds("item-selected-Sent").expect("Sent");
    cx.simulate_click(row.center(), Modifiers::none());
    settle(cx, |_| true);
    cx.simulate_keystrokes("shift-f10");
    settle(cx, |_| true);
    assert!(cx.debug_bounds("menu-row-Pin to top").is_some());
    cx.simulate_keystrokes("escape");
    settle(cx, |_| true);
    // As the tray: a new window around the same mail.
    let again = cx.update(|_, app| app.open_window(gpui::WindowOptions::default(), |_, _| mail.clone())).unwrap();
    cx.update(|window, _| window.remove_window());
    let mut cx = VisualTestContext::from_window(again.into(), cx);
    cx.simulate_resize(size(px(1100.), px(800.)));
    settle(&mut cx, |_| true);
    assert!(cx.debug_bounds("item-selected-Sent").is_some(), "the side is drawn");
    assert!(cx.debug_bounds("menu-row-Pin to top").is_none(), "no menu comes back");
}

#[test]
fn a_right_click_off_the_folders_opens_no_menu() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let (_, cx, _, _) = opened(dir.path(), &mut cx);
    let title = cx.debug_bounds("fold ann@example.com").expect("the account title");
    cx.simulate_mouse_down(title.center(), MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(title.center(), MouseButton::Right, Modifiers::none());
    settle(cx, |_| true);
    assert!(cx.debug_bounds("menu-panel-0").is_none(), "no empty menu");
}

#[test]
fn a_chinese_folder_reads_in_chinese() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let folders = [("INBOX", Some(Role::Inbox)), ("Zebra", None), ("&UXZO1mWHTvZZOQ-", None)];
    let [_, zebra, other]: [MailboxId; 3] = store.set_folders(ann, &folders).unwrap().try_into().unwrap();
    drop(store);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    settle(cx, |app| mail.read(app).syncs.get(&ann).is_some_and(|each| each.folders.len() == 3));
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(Open::Mailbox(ann, zebra), cx)));
    settle(cx, |_| true);
    let zebra = cx.debug_bounds("item-selected-Zebra").expect("Zebra");
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(Open::Mailbox(ann, other), cx)));
    settle(cx, |_| true);
    let other = cx.debug_bounds("item-selected-其他文件夹").expect("其他文件夹");
    // Read order: Z before 其; the raw & sorts first.
    assert!(other.top() > zebra.top(), "{other:?} under {zebra:?}");
}

#[test]
fn only_accounts_fold() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let (_, cx, _, _) = opened(dir.path(), &mut cx);
    assert!(cx.debug_bounds("fold ann@example.com").is_some());
    assert!(cx.debug_bounds("fold Unified").is_none());
}
