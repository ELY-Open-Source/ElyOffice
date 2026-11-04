//! Contacts: the page, vCard files, and addresses offered (MAIL-12).

use std::time::{Duration, Instant};

use ely_mail_engine::Store;
use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext, px, size};

use super::{Keys, account, settle, window, writing::opened};
use crate::{boxes::Open, mail::Mail};

/// The page's list once read, by address.
fn listed(mail: &Entity<Mail>, cx: &mut VisualTestContext) -> Option<Vec<String>> {
    cx.update(|_, app| {
        let people = mail.read(app).people.as_ref()?;
        let list = people.list.as_ref()?.as_ref().ok()?;
        Some(list.iter().map(|each| each.address.clone()).collect())
    })
}

fn click(selector: &'static str, cx: &mut VisualTestContext) {
    let at = cx.debug_bounds(selector).unwrap_or_else(|| panic!("{selector} is laid out"));
    cx.simulate_click(at.center(), Modifiers::none());
}

fn waited(path: &std::path::Path, cx: &mut VisualTestContext) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !path.exists() && Instant::now() < deadline {
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn the_page_adds_removes_exports_and_imports() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store.add_account(&account("me@example.com")).unwrap();
    let bo = store.add_contact("Bo", "bo@example.com").unwrap();
    drop(store);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.simulate_resize(size(px(1100.), px(800.)));
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(Open::Contacts, cx)));
    settle(cx, |app| mail.read(app).people.as_ref().is_some_and(|people| people.list.is_some()));
    assert_eq!(listed(&mail, cx).unwrap(), ["bo@example.com"]);
    click("mail-contact-name", cx);
    cx.simulate_input("Ann Ross");
    click("mail-contact-address", cx);
    cx.simulate_input("ann@example.com");
    click("mail-contact-add", cx);
    settle(cx, |app| {
        let list = mail.read(app).people.as_ref().and_then(|people| people.list.as_ref());
        list.is_some_and(|list| list.as_ref().is_ok_and(|list| list.len() == 2))
    });
    let card = dir.path().join("people.vcf");
    click("mail-contacts-export", cx);
    cx.run_until_parked();
    let chosen = card.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    waited(&card, cx);
    let written = std::fs::read_to_string(&card).unwrap();
    assert!(written.contains("FN:Ann Ross\r\n") && written.contains("EMAIL;TYPE=INTERNET:bo@example.com\r\n"));
    let remove: &'static str = format!("mail-contact-remove-{}", bo.0).leak();
    click(remove, cx);
    settle(cx, |app| {
        let list = mail.read(app).people.as_ref().and_then(|people| people.list.as_ref());
        list.is_some_and(|list| list.as_ref().is_ok_and(|list| list.len() == 1))
    });
    assert_eq!(listed(&mail, cx).unwrap(), ["ann@example.com"]);
    click("mail-contacts-import", cx);
    cx.run_until_parked();
    cx.simulate_path_prompt_response(move |_| Some(vec![card]));
    settle(cx, |app| {
        let list = mail.read(app).people.as_ref().and_then(|people| people.list.as_ref());
        list.is_some_and(|list| list.as_ref().is_ok_and(|list| list.len() == 2))
    });
    let kept = Store::open(dir.path()).unwrap().contacts().unwrap();
    let kept: Vec<_> = kept.into_iter().map(|each| (each.name, each.address)).collect();
    assert_eq!(
        kept,
        [(Some("Ann Ross".into()), "ann@example.com".into()), (Some("Bo".into()), "bo@example.com".into())]
    );
}

#[test]
fn a_new_message_offers_the_contacts() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store.add_account(&account("me@example.com")).unwrap();
    store.add_contact("Bo", "bo@example.com").unwrap();
    store.add_contact("", "cy@example.com").unwrap();
    drop(store);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.update(|_, app| mail.update(app, |mail, cx| mail.write_new(cx)));
    let compose = opened(cx);
    settle(cx, |app| compose.read(app).is_ok_and(|compose| compose.offered.len() == 2));
    let offered = cx.update(|_, app| {
        let compose = compose.read(app).unwrap();
        compose.offered.iter().map(|each| (each.name.to_string(), each.address.to_string())).collect::<Vec<_>>()
    });
    assert_eq!(offered, [("Bo".into(), "bo@example.com".into()), (String::new(), "cy@example.com".into())]);
}

#[test]
fn contacts_are_kept_with_no_account() {
    let dir = tempfile::tempdir().unwrap();
    Store::open(dir.path()).unwrap().add_contact("Bo", "bo@example.com").unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.simulate_resize(size(px(1100.), px(800.)));
    cx.run_until_parked();
    assert!(cx.update(|_, app| mail.read(app).accounts().is_empty()));
    assert!(cx.debug_bounds("mailbox-list").is_some(), "the side shows with no account");
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(Open::Contacts, cx)));
    settle(cx, |app| mail.read(app).people.as_ref().is_some_and(|people| people.list.is_some()));
    assert_eq!(listed(&mail, cx).unwrap(), ["bo@example.com"]);
    assert!(cx.debug_bounds("mail-contact-add").is_some(), "the page shows, not the empty state");
}
