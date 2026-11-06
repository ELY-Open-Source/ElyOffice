mod queue;

use ely_mail_engine::{Account, Change, Key, Secret, Security, push::Dropped, sync};
use fake::{certs, expect, send, serve};
use queue::{caps, change, dropped, here, listed, present, push, select, steps, waiting};

#[tokio::test]
async fn nothing_queued_sends_nothing() {
    let (here, _) = here(&[1]);
    assert_eq!(push(&here, caps("")).await.unwrap(), []);
}

#[tokio::test]
async fn flags_go_to_the_server_once() {
    let (here, ids) = here(&[1, 2]);
    change(&here, &ids, Change::Seen(true));
    change(&here, &ids[1..], Change::Flagged(true));
    let script = steps(vec![
        caps(""),
        select(7, 2),
        present("1:2", &[1, 2]),
        vec![
            expect("{tag} UID STORE 1:2 +FLAGS.SILENT (\\Seen)"),
            send("{tag} OK stored\n"),
            expect("{tag} UID STORE 2 +FLAGS.SILENT (\\Flagged)"),
            send("{tag} OK stored\n"),
        ],
    ]);
    let pushed = push(&here, script).await.unwrap();
    assert_eq!(pushed, []);
    assert_eq!(waiting(&here), 0);
    assert_eq!(push(&here, caps("")).await.unwrap(), []);
}

#[tokio::test]
async fn a_message_gone_from_the_server_is_let_go() {
    let (here, ids) = here(&[1, 2]);
    change(&here, &ids, Change::Seen(true));
    let script = steps(vec![
        caps(""),
        select(7, 1),
        present("1:2", &[2]),
        vec![
            expect("{tag} UID STORE 2 +FLAGS.SILENT (\\Seen)"),
            send("{tag} OK stored\n"),
        ],
    ]);
    let pushed = push(&here, script).await.unwrap();
    assert_eq!(pushed, [Dropped::Gone]);
    // Not retried: the next push has nothing.
    assert_eq!(waiting(&here), 0);
}

#[tokio::test]
async fn a_renumbered_folder_lets_its_changes_go() {
    let (here, ids) = here(&[1]);
    change(&here, &ids, Change::Move(here.work));
    let script = steps(vec![caps("MOVE"), select(8, 1)]);
    let pushed = push(&here, script).await.unwrap();
    assert_eq!(pushed, [Dropped::Renumbered]);
    assert_eq!(waiting(&here), 0);
    assert_eq!(listed(&here, here.inbox), ids);
}

#[tokio::test]
async fn a_move_takes_its_new_uid_and_keeps_its_body() {
    let (here, ids) = here(&[1, 2]);
    change(&here, &ids, Change::Move(here.work));
    let script = steps(vec![
        caps("MOVE UIDPLUS"),
        select(7, 2),
        present("1:2", &[1, 2]),
        vec![
            expect("{tag} UID MOVE 1:2 \"Work\""),
            send("* OK [COPYUID 7 1:2 40:41] moved\n* 1 EXPUNGE\n* 1 EXPUNGE\n{tag} OK done\n"),
        ],
    ]);
    assert_eq!(push(&here, script).await.unwrap(), []);
    let store = here.store.lock().unwrap();
    let key = |uid| Key::Imap {
        uidvalidity: 7,
        uid,
    };
    assert_eq!(store.find(here.work, &key(40)).unwrap(), Some(ids[0]));
    assert_eq!(store.find(here.work, &key(41)).unwrap(), Some(ids[1]));
    assert!(store.get(ids[0]).unwrap().body);
    drop(store);
    assert_eq!(listed(&here, here.work), [ids[1], ids[0]]);
    assert_eq!(waiting(&here), 0);
}

#[tokio::test]
async fn a_move_without_uids_waits_for_the_target_sync() {
    let (here, ids) = here(&[1]);
    change(&here, &ids, Change::Move(here.work));
    let script = steps(vec![
        caps("MOVE"),
        select(7, 1),
        present("1", &[1]),
        vec![
            expect("{tag} UID MOVE 1 \"Work\""),
            send("* 1 EXPUNGE\n{tag} OK done\n"),
        ],
    ]);
    assert_eq!(push(&here, script).await.unwrap(), []);
    assert!(here.store.lock().unwrap().get(ids[0]).is_err());
    assert_eq!(listed(&here, here.work), []);
}

#[tokio::test]
async fn without_move_a_copy_and_expunge_by_uid() {
    let (here, ids) = here(&[3]);
    change(&here, &ids, Change::Move(here.work));
    let script = steps(vec![
        caps("UIDPLUS"),
        select(7, 1),
        present("3", &[3]),
        vec![
            expect("{tag} UID COPY 3 \"Work\""),
            send("{tag} OK [COPYUID 7 3 9] copied\n"),
            expect("{tag} UID STORE 3 +FLAGS.SILENT (\\Deleted)"),
            send("{tag} OK stored\n"),
            expect("{tag} UID EXPUNGE 3"),
            send("* 1 EXPUNGE\n{tag} OK gone\n"),
        ],
    ]);
    assert_eq!(push(&here, script).await.unwrap(), []);
    let key = Key::Imap {
        uidvalidity: 7,
        uid: 9,
    };
    let store = here.store.lock().unwrap();
    assert_eq!(store.find(here.work, &key).unwrap(), Some(ids[0]));
}

#[tokio::test]
async fn a_server_that_cannot_move_one_keeps_it_in_place() {
    let (here, ids) = here(&[1]);
    change(&here, &ids, Change::Move(here.work));
    let script = steps(vec![caps(""), select(7, 1), present("1", &[1])]);
    let pushed = push(&here, script).await.unwrap();
    assert_eq!(pushed, [Dropped::Unsupported("MOVE".into())]);
    assert_eq!(listed(&here, here.inbox), ids);
    assert_eq!(waiting(&here), 0);
}

#[tokio::test]
async fn a_refused_flag_is_let_go_with_the_server_words() {
    let (here, ids) = here(&[1]);
    change(&here, &ids, Change::Flagged(true));
    let script = steps(vec![
        caps(""),
        select(7, 1),
        present("1", &[1]),
        vec![
            expect("{tag} UID STORE 1 +FLAGS.SILENT (\\Flagged)"),
            send("{tag} NO read-only folder\n"),
            expect("{tag} UID FETCH 1 (UID FLAGS)"),
            send("* 1 FETCH (UID 1 FLAGS (\\Seen))\n{tag} OK flags\n"),
        ],
    ]);
    let pushed = push(&here, script).await.unwrap();
    assert_eq!(pushed, [Dropped::Refused("NO read-only folder".into())]);
    assert_eq!(waiting(&here), 0);
    // Undone here: shown as the server has it.
    let flags = here.store.lock().unwrap().get(ids[0]).unwrap().flags;
    assert_eq!(flags, Vec::<String>::new());
}

#[tokio::test]
async fn a_refused_flag_unread_back_returns_to_the_servers_last() {
    let (here, ids) = here(&[1, 2]);
    change(&here, &ids, Change::Flagged(true));
    // The server flagged one meanwhile; a sync said so.
    let flagged = &["\\Flagged"][..];
    here.store
        .lock()
        .unwrap()
        .set_flags(ids[1], flagged)
        .unwrap();
    let refused = "{tag} NO cannot read flags\n";
    let script = steps(vec![
        caps(""),
        select(7, 2),
        present("1:2", &[1, 2]),
        vec![
            expect("{tag} UID STORE 1:2 +FLAGS.SILENT (\\Flagged)"),
            send("{tag} NO read-only folder\n"),
            expect("{tag} UID FETCH 1:2 (UID FLAGS)"),
            send(refused),
        ],
    ]);
    push(&here, script).await.unwrap();
    assert_eq!(waiting(&here), 0);
    let flags = |id| here.store.lock().unwrap().get(id).unwrap().flags;
    assert_eq!(flags(ids[0]), Vec::<String>::new());
    assert_eq!(flags(ids[1]), flagged);
}

#[tokio::test]
async fn a_refused_flag_keeps_a_newer_one() {
    let (here, ids) = here(&[1]);
    change(&here, &ids, Change::Flagged(true));
    let (store, id) = (here.store.clone(), ids[0]);
    let script = steps(vec![
        caps(""),
        select(7, 1),
        present("1", &[1]),
        vec![
            expect("{tag} UID STORE 1 +FLAGS.SILENT (\\Flagged)"),
            fake::Step::Run(Box::new(move || {
                let change = Change::Seen(true);
                store.lock().unwrap().change(&[id], change).unwrap();
            })),
            send("{tag} NO read-only folder\n"),
            expect("{tag} UID FETCH 1 (UID FLAGS)"),
            send("* 1 FETCH (UID 1 FLAGS ())\n{tag} OK flags\n"),
        ],
    ]);
    push(&here, script).await.unwrap();
    // The newer change goes next time; nothing undone.
    assert_eq!(waiting(&here), 1);
    let flags = here.store.lock().unwrap().get(id).unwrap().flags;
    assert_eq!(flags, ["\\Flagged", "\\Seen"]);
}

#[tokio::test]
async fn delete_for_good_expunges_by_uid() {
    let (here, ids) = here(&[5]);
    change(&here, &ids, Change::Delete { erase: true });
    let script = steps(vec![
        caps("UIDPLUS"),
        select(7, 1),
        present("5", &[5]),
        vec![
            expect("{tag} UID STORE 5 +FLAGS.SILENT (\\Deleted)"),
            send("{tag} OK stored\n"),
            expect("{tag} UID EXPUNGE 5"),
            send("* 1 EXPUNGE\n{tag} OK gone\n"),
        ],
    ]);
    assert_eq!(push(&here, script).await.unwrap(), []);
    assert_eq!(waiting(&here), 0);
}

#[tokio::test]
async fn a_lost_connection_keeps_the_change_for_the_next_sync() {
    let (here, ids) = here(&[1]);
    change(&here, &ids, Change::Seen(true));
    let certs = certs();
    let script = vec![
        send("* OK ready\n"),
        expect("{tag} LOGIN \"ann@example.com\" \"pw\""),
        send("{tag} OK done\n"),
        expect("{tag} CAPABILITY"),
    ];
    let (incoming, _task) = serve(&certs, Security::Tls, script).await;
    let account = Account {
        incoming,
        ..fake::account("ann@example.com")
    };
    let secret = Secret::Password("pw".into());
    let done = sync::changes(&account, here.id, secret, &certs.trusted, &here.store).await;
    assert!(done.is_err());
    assert_eq!(waiting(&here), 1);
}

#[tokio::test]
async fn a_change_made_while_pushing_waits_for_the_next_push() {
    let (here, ids) = here(&[1]);
    change(&here, &ids, Change::Seen(true));
    let (store, id) = (here.store.clone(), ids[0]);
    let script = steps(vec![
        caps(""),
        select(7, 1),
        present("1", &[1]),
        vec![
            expect("{tag} UID STORE 1 +FLAGS.SILENT (\\Seen)"),
            fake::Step::Run(Box::new(move || {
                let change = Change::Flagged(true);
                store.lock().unwrap().change(&[id], change).unwrap();
            })),
            send(
                "{tag} OK stored
",
            ),
        ],
    ]);
    assert_eq!(push(&here, script).await.unwrap(), []);
    assert_eq!(waiting(&here), 1);
}

#[tokio::test]
async fn moved_back_while_moving_it_moves_back_from_its_new_place() {
    let (here, ids) = here(&[1]);
    change(&here, &ids, Change::Move(here.work));
    let (store, id, inbox) = (here.store.clone(), ids[0], here.inbox);
    let script = steps(vec![
        caps("MOVE UIDPLUS"),
        select(7, 1),
        present("1", &[1]),
        vec![
            expect("{tag} UID MOVE 1 \"Work\""),
            fake::Step::Run(Box::new(move || {
                store
                    .lock()
                    .unwrap()
                    .change(&[id], Change::Move(inbox))
                    .unwrap();
            })),
            send(
                "{tag} OK [COPYUID 7 1 40] moved
",
            ),
        ],
    ]);
    assert_eq!(push(&here, script).await.unwrap(), []);
    let key = Key::Imap {
        uidvalidity: 7,
        uid: 40,
    };
    assert_eq!(
        here.store.lock().unwrap().find(here.work, &key).unwrap(),
        Some(id)
    );
    // Shown where it was wanted; the move back is queued.
    assert_eq!(listed(&here, here.inbox), [id]);
    assert_eq!(waiting(&here), 1);
    let script = steps(vec![
        caps("MOVE UIDPLUS"),
        vec![
            expect("{tag} SELECT \"Work\""),
            send(
                "* 1 EXISTS
* OK [UIDVALIDITY 7] v
{tag} OK in
",
            ),
        ],
        present("40", &[40]),
        vec![
            expect("{tag} UID MOVE 40 \"INBOX\""),
            send(
                "{tag} OK [COPYUID 7 40 2] moved
",
            ),
        ],
    ]);
    assert_eq!(push(&here, script).await.unwrap(), []);
    let back = Key::Imap {
        uidvalidity: 7,
        uid: 2,
    };
    assert_eq!(
        here.store.lock().unwrap().find(here.inbox, &back).unwrap(),
        Some(id)
    );
    assert_eq!(waiting(&here), 0);
}

#[tokio::test]
async fn a_sync_sends_changes_first_and_keeps_one_made_meanwhile() {
    let (here, ids) = here(&[1]);
    change(&here, &ids, Change::Seen(true));
    let (store, id) = (here.store.clone(), ids[0]);
    let script = steps(vec![
        caps(""),
        select(7, 1),
        present("1", &[1]),
        vec![
            expect("{tag} UID STORE 1 +FLAGS.SILENT (\\Seen)"),
            send("{tag} OK stored\n"),
            expect("{tag} LIST \"\" *"),
            send("* LIST () \"/\" INBOX\n{tag} OK listed\n"),
            expect("{tag} EXAMINE \"INBOX\""),
            send("* 1 EXISTS\n* OK [UIDVALIDITY 7] v\n{tag} OK in\n"),
            fake::Step::Run(Box::new(move || {
                let change = Change::Flagged(true);
                store.lock().unwrap().change(&[id], change).unwrap();
            })),
            expect("{tag} UID FETCH 1:* (UID FLAGS)"),
            send("* 1 FETCH (UID 1 FLAGS (\\Seen))\n{tag} OK flags\n"),
            expect("{tag} UID SEARCH SINCE 21-Jul-2026"),
            send("* SEARCH\n{tag} OK found\n"),
        ],
    ]);
    let certs = certs();
    let mut full = vec![
        send("* OK ready\n"),
        expect("{tag} LOGIN \"ann@example.com\" \"pw\""),
        send("{tag} OK done\n"),
    ];
    full.extend(script);
    full.push(expect("{tag} LOGOUT"));
    full.push(send("* BYE\n{tag} OK bye\n"));
    let (incoming, task) = serve(&certs, Security::Tls, full).await;
    let account = Account {
        incoming,
        ..fake::account("ann@example.com")
    };
    let secret = Secret::Password("pw".into());
    // 2026-10-19 12:00 UTC; bodies from 21 July.
    let now = 1_792_411_200;
    let report = sync::imap(
        &account,
        here.id,
        secret,
        &certs.trusted,
        &here.store,
        now,
        &|| {},
    )
    .await;
    task.await.unwrap().unwrap();
    report.unwrap();
    assert_eq!(dropped(&here), []);
    // The flag made mid-sync is not undone by the server's.
    let flags = here.store.lock().unwrap().get(id).unwrap().flags;
    assert_eq!(flags, ["\\Flagged", "\\Seen"]);
    assert_eq!(waiting(&here), 1);
}

#[tokio::test]
async fn a_change_undone_before_it_went_sends_nothing() {
    let (here, ids) = here(&[1]);
    change(&here, &ids, Change::Move(here.work));
    change(&here, &ids, Change::Move(here.inbox));
    assert_eq!(push(&here, caps("")).await.unwrap(), []);
    assert_eq!(listed(&here, here.inbox), ids);
}

#[tokio::test]
async fn deleting_for_good_needs_uidplus() {
    let (here, ids) = here(&[5]);
    change(&here, &ids, Change::Delete { erase: true });
    let script = steps(vec![caps(""), select(7, 1), present("5", &[5])]);
    let pushed = push(&here, script).await.unwrap();
    assert_eq!(pushed, [Dropped::Unsupported("UIDPLUS".into())]);
    assert_eq!(waiting(&here), 0);
}
