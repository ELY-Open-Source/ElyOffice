use ely_mail_engine::{Action, Match, Role, Store};
use gpui::{TestAppContext, px, size};

use super::{Keys, account, settle, window};
use crate::boxes::Open;

#[test]
fn the_account_page_adds_and_removes_rules() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let [_, receipts] = store
        .set_folders(ann, &[("INBOX", Some(Role::Inbox)), ("Receipts", None)])
        .unwrap()
        .try_into()
        .unwrap();
    let flag = store.add_rule(ann, Match::Subject, "urgent", Action::Flag).unwrap();
    drop(store);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(Open::Account(ann), cx)));
    cx.simulate_resize(size(px(1100.), px(1400.)));
    settle(cx, |app| {
        let mail = mail.read(app);
        let read = mail.ruling.as_ref().and_then(|ruling| ruling.rules.as_ref());
        read.is_some_and(|rules| rules.as_ref().is_ok_and(|rules| rules.len() == 1))
            && mail.syncs.get(&ann).is_some_and(|each| each.folders.len() == 2)
    });
    let row: &'static str = format!("mail-rule-{}", flag.0).leak();
    assert!(cx.debug_bounds(row).is_some(), "the stored rule is listed");
    // A rule moving mail from the shop to Receipts.
    cx.update(|_, app| {
        mail.update(app, |mail, _| {
            let ruling = mail.ruling.as_mut().unwrap();
            ruling.field = Match::From;
            ruling.action = receipts.0.to_string().into();
        })
    });
    let words = cx.debug_bounds("mail-rule-words").expect("the words box is laid out");
    cx.simulate_click(words.center(), gpui::Modifiers::none());
    cx.simulate_input("Shop.example");
    let add = cx.debug_bounds("mail-rule-add").expect("the add button is laid out");
    cx.simulate_click(add.center(), gpui::Modifiers::none());
    settle(cx, |app| {
        let read = mail.read(app).ruling.as_ref().and_then(|ruling| ruling.rules.as_ref());
        read.is_some_and(|rules| rules.as_ref().is_ok_and(|rules| rules.len() == 2))
    });
    let stored = Store::open(dir.path()).unwrap().rules(ann).unwrap();
    let made: Vec<_> = stored.iter().map(|rule| (rule.field, rule.words.as_str(), rule.action)).collect();
    assert_eq!(
        made,
        [(Match::Subject, "urgent", Action::Flag), (Match::From, "shop.example", Action::Move(receipts))]
    );
    let remove: &'static str = format!("mail-rule-remove-{}", flag.0).leak();
    let remove = cx.debug_bounds(remove).expect("the remove button is laid out");
    cx.simulate_click(remove.center(), gpui::Modifiers::none());
    settle(cx, |app| {
        let read = mail.read(app).ruling.as_ref().and_then(|ruling| ruling.rules.as_ref());
        read.is_some_and(|rules| rules.as_ref().is_ok_and(|rules| rules.len() == 1))
    });
    let left = Store::open(dir.path()).unwrap().rules(ann).unwrap();
    assert_eq!(left.iter().map(|rule| rule.words.as_str()).collect::<Vec<_>>(), ["shop.example"]);
}

#[test]
fn a_rule_saved_after_leaving_its_page_leaves_the_new_page_be() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let bo = store.add_account(&account("bo@example.com")).unwrap();
    store.add_rule(bo, Match::From, "shop", Action::Flag).unwrap();
    drop(store);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(Open::Account(ann), cx)));
    settle(cx, |app| mail.read(app).ruling.as_ref().is_some_and(|ruling| ruling.rules.is_some()));
    cx.update(|_, app| {
        mail.update(app, |mail, cx| {
            mail.add_rule("urgent".into(), cx);
            mail.open_box(Open::Account(bo), cx);
        })
    });
    settle(cx, |app| {
        let ruling = mail.read(app).ruling.as_ref();
        ruling.is_some_and(|ruling| ruling.account == bo && ruling.rules.as_ref().is_some_and(|rules| rules.is_ok()))
    });
    // Ann's save landing must not take Bo's page back.
    for _ in 0..5 {
        cx.run_until_parked();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let shown = cx.update(|_, app| {
        let ruling = mail.read(app).ruling.as_ref().unwrap();
        let rules = ruling.rules.as_ref().unwrap().as_ref().unwrap();
        (ruling.account, rules.iter().map(|rule| rule.words.clone()).collect::<Vec<_>>())
    });
    assert_eq!(shown, (bo, vec!["shop".to_owned()]));
    assert_eq!(Store::open(dir.path()).unwrap().rules(ann).unwrap()[0].words, "urgent");
}

#[test]
fn rules_move_only_to_server_folders() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    store.set_folders(ann, &[("INBOX", Some(Role::Inbox)), ("Receipts", None)]).unwrap();
    store.import_mbox(ann, "Old", std::io::Cursor::new(b"")).unwrap();
    let folders = store.folders(ann).unwrap();
    assert!(folders.iter().any(|folder| folder.local), "an imported folder is there");
    let mut cx = TestAppContext::single();
    let (_, vcx) = window(&mut cx, dir.path(), Keys::default());
    let offered = vcx.update(|_, app| crate::rules::rule_actions(ely_mail_engine::Protocol::Imap, &folders, app));
    let named: Vec<_> = offered.iter().map(|choice| choice.label.to_string()).collect();
    assert_eq!(named, ["flag it", "move it to INBOX", "move it to Receipts"], "no Archive folder, no archiving");
    store.set_folders(ann, &[("INBOX", Some(Role::Inbox)), ("All Mail", Some(Role::All))]).unwrap();
    let folders = store.folders(ann).unwrap();
    let offered = vcx.update(|_, app| crate::rules::rule_actions(ely_mail_engine::Protocol::Imap, &folders, app));
    let named: Vec<_> = offered.iter().map(|choice| choice.label.to_string()).collect();
    assert_eq!(named, ["flag it", "archive it", "move it to All Mail", "move it to INBOX"]);
}

#[test]
fn a_saved_move_names_its_folder_in_chinese() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let [_, drafts] = store
        .set_folders(ann, &[("INBOX", Some(Role::Inbox)), ("&g0l6P3ux-", None)])
        .unwrap()
        .try_into()
        .unwrap();
    store.add_rule(ann, Match::From, "shop", Action::Move(drafts)).unwrap();
    drop(store);
    let rules = Store::open(dir.path()).unwrap().rules(ann).unwrap();
    let mut cx = TestAppContext::single();
    let (_, vcx) = window(&mut cx, dir.path(), Keys::default());
    let said = vcx.update(|_, app| crate::rules::said(&rules[0], app).to_string());
    assert!(said.ends_with("move it to 草稿箱"), "{said}");
    let folders = Store::open(dir.path()).unwrap().folders(ann).unwrap();
    let offered = vcx.update(|_, app| crate::rules::rule_actions(ely_mail_engine::Protocol::Imap, &folders, app));
    let moves = offered.iter().find(|choice| choice.value == drafts.0.to_string()).expect("a move to 草稿箱");
    assert_eq!(moves.label.as_ref(), "move it to 草稿箱");
}
