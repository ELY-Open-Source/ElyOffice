use ely_mail_engine::{Action, Key, Match, Role, Security, Store};
use fake::{Step, expect, send};
use gpui::{TestAppContext, px, size};

use super::{
    Keys, account, closed_port,
    drafts::rooted,
    outbox::with_password,
    settle, window, window_with,
    whole::{bo, ended, runtime},
};
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

/// Two new messages' headers, read and undated.
fn arrived() -> String {
    let mut lines = String::new();
    for (at, uid, subject) in [(1, 2, "urgent: call"), (2, 3, "Your receipt")] {
        // Sent lines gain CR; the literal counts them.
        let head = format!("Subject: {subject}\nFrom: a@example.com\n\n");
        let size = head.replace('\n', "\r\n").len();
        lines.push_str(&format!("* {at} FETCH (UID {uid} FLAGS (\\Seen) BODY[HEADER] {{{size}}}\n{head})\n"));
    }
    lines.push_str("{tag} OK headers\n");
    lines
}

/// Logged in as Bo, then `rest`, then out.
fn session(rest: Vec<Step>) -> Vec<Step> {
    let mut script = vec![
        send("* OK ready\n"),
        expect("{tag} LOGIN \"bo@example.com\" \"pw\""),
        send("{tag} OK done\n"),
    ];
    script.extend(rest);
    script.push(expect("{tag} LOGOUT"));
    script.push(send("* BYE\n{tag} OK bye\n"));
    script
}

#[test]
fn a_sync_tells_a_refused_rule_and_sends_what_rules_did() {
    let rt = runtime();
    let certs = fake::certs();
    let (open, gate) = std::sync::mpsc::channel::<()>();
    let mut synced = vec![Step::Run(Box::new(move || gate.recv().unwrap()))];
    synced.extend(session(vec![
        expect("{tag} CAPABILITY"),
        send("* CAPABILITY IMAP4rev1\n{tag} OK caps\n"),
        expect("{tag} LIST \"\" *"),
        send("* LIST (\\HasNoChildren) \"/\" INBOX\n{tag} OK listed\n"),
        expect("{tag} EXAMINE \"INBOX\""),
        send("* 2 EXISTS\n* OK [UIDVALIDITY 7] v\n{tag} OK in\n"),
        expect("{tag} UID FETCH 1:* (UID FLAGS)"),
        send("* 1 FETCH (UID 2 FLAGS (\\Seen))\n* 2 FETCH (UID 3 FLAGS (\\Seen))\n{tag} OK flags\n"),
        Step::Prefix("{tag} UID SEARCH SINCE".into()),
        send("* SEARCH\n{tag} OK found\n"),
        expect("{tag} UID FETCH 2:3 (UID FLAGS BODY.PEEK[HEADER])"),
        send(&arrived()),
    ]));
    let pushed = session(vec![
        expect("{tag} CAPABILITY"),
        send("* CAPABILITY IMAP4rev1\n{tag} OK caps\n"),
        expect("{tag} SELECT \"INBOX\""),
        send("* 2 EXISTS\n* OK [UIDVALIDITY 7] v\n{tag} OK [READ-WRITE] in\n"),
        expect("{tag} UID FETCH 2 (UID)"),
        send("* 1 FETCH (UID 2)\n{tag} OK fetched\n"),
        expect("{tag} UID STORE 2 +FLAGS.SILENT (\\Flagged)"),
        send("{tag} OK stored\n"),
    ]);
    let (server, task) = rt.block_on(fake::serve_each(&certs, Security::Tls, vec![synced, pushed], None));
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let id = store.add_account(&bo(server.port, closed_port())).unwrap();
    let folders = [("INBOX", Some(Role::Inbox)), ("Archive", Some(Role::Archive))];
    let [inbox, _] = store.set_folders(id, &folders).unwrap().try_into().unwrap();
    // Synced before: what comes now is news.
    store.set_synced(id, 1).unwrap();
    store.add_rule(id, Match::Subject, "urgent", Action::Flag).unwrap();
    // The server drops Archive before the next sync.
    store.add_rule(id, Match::Subject, "receipt", Action::Archive).unwrap();
    drop(store);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window_with(&mut cx, dir.path(), with_password(), Ok(certs.trusted.clone()));
    let root = rooted(&mail, cx);
    open.send(()).unwrap();
    settle(cx, |app| task.is_finished() && mail.read(app).syncs[&id].task.is_none());
    ended(&rt, task);
    let toasts = cx.update(|_, app| root.read(app).toasts(app));
    assert!(toasts.iter().any(|(words, _)| words.starts_with("A rule could not act:")), "{toasts:?}");
    let store = Store::open(dir.path()).unwrap();
    let key = Key::Imap { uidvalidity: 7, uid: 2 };
    let flagged = store.get(store.find(inbox, &key).unwrap().unwrap()).unwrap().flags;
    assert!(flagged.contains(&"\\Flagged".to_owned()), "{flagged:?}");
    assert_eq!(store.synced(id).unwrap().waiting, 0);
}
