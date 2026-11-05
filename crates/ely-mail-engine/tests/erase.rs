mod queue;

use ely_mail_engine::{Change, Fetched, Key, Role, push::Dropped};
use fake::{expect, send};
use queue::{Here, caps, change, here, listed, present, push, select, steps, waiting};

const MAIL: &[u8] = b"From: a@example.com\r\nSubject: Hi\r\n\r\nhello\r\n";

#[tokio::test]
async fn a_refused_erase_brings_the_message_back() {
    let (here, ids) = here(&[5]);
    change(&here, &ids, Change::Delete { erase: true });
    assert_eq!(listed(&here, here.inbox), []);
    let script = steps(vec![
        caps("UIDPLUS"),
        select(7, 1),
        present("5", &[5]),
        vec![
            expect("{tag} UID STORE 5 +FLAGS.SILENT (\\Deleted)"),
            send("{tag} NO read-only folder\n"),
        ],
    ]);
    let pushed = push(&here, script).await.unwrap();
    assert_eq!(pushed, [Dropped::Refused("NO read-only folder".into())]);
    assert_eq!(waiting(&here), 0);
    assert_eq!(listed(&here, here.inbox), ids);
    // The original came back with it.
    let original = here.store.lock().unwrap().original(ids[0]).unwrap();
    assert_eq!(original.as_deref(), Some(MAIL));
}

#[tokio::test]
async fn an_erase_the_server_cannot_do_brings_it_back() {
    let (here, ids) = here(&[5]);
    change(&here, &ids, Change::Delete { erase: true });
    let script = steps(vec![caps(""), select(7, 1), present("5", &[5])]);
    let pushed = push(&here, script).await.unwrap();
    assert_eq!(pushed, [Dropped::Unsupported("UIDPLUS".into())]);
    assert_eq!(listed(&here, here.inbox), ids);
}

#[tokio::test]
async fn an_erased_message_goes_once_the_server_expunges_it() {
    let (here, ids) = here(&[5]);
    change(&here, &ids, Change::Delete { erase: true });
    let unread = here.store.lock().unwrap().unread().unwrap();
    assert_eq!(unread.get(&here.inbox), None);
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
    let store = here.store.lock().unwrap();
    assert!(store.get(ids[0]).is_err());
    // Nothing left to bring back.
    let key = Key::Imap {
        uidvalidity: 7,
        uid: 5,
    };
    assert_eq!(store.find(here.inbox, &key).unwrap(), None);
}

#[test]
fn a_destination_no_longer_listed_lets_the_move_go_and_keeps_the_flag() {
    let (here, ids) = here(&[1]);
    change(&here, &ids, Change::Flagged(true));
    change(&here, &ids, Change::Move(here.work));
    let mut store = here.store.lock().unwrap();
    store
        .set_folders(here.id, &[("INBOX", Some(Role::Inbox))])
        .unwrap();
    assert_eq!(store.synced(here.id).unwrap().waiting, 1);
    assert_eq!(
        store.take_dropped(here.id).unwrap(),
        [Dropped::Vanished("Work".into())]
    );
    // The server's old flags do not undo the star.
    let key = Key::Imap {
        uidvalidity: 7,
        uid: 1,
    };
    store
        .put(here.inbox, &key, &[], Fetched::Headers(MAIL))
        .unwrap();
    assert_eq!(store.get(ids[0]).unwrap().flags, ["\\Flagged"]);
    drop(store);
    assert_eq!(listed(&here, here.inbox), ids);
}

fn flags(here: &Here, uid: u32) -> Vec<String> {
    let store = here.store.lock().unwrap();
    let key = Key::Imap {
        uidvalidity: 7,
        uid,
    };
    let id = store.find(here.inbox, &key).unwrap().unwrap();
    store.get(id).unwrap().flags
}

fn refused_select() -> Vec<fake::Step> {
    vec![
        expect("{tag} SELECT \"INBOX\""),
        send("{tag} NO [UNAVAILABLE] try later\n"),
    ]
}

fn refused_check() -> Vec<fake::Step> {
    let mut script = select(7, 1);
    script.push(expect("{tag} UID FETCH 5 (UID)"));
    script.push(send("{tag} NO busy\n"));
    script
}

#[tokio::test]
async fn a_refused_folder_brings_an_erase_back() {
    for (refusal, words) in [(refused_select(), "NO [UNAVAILABLE] try later"), (refused_check(), "NO busy")] {
        let (here, ids) = here(&[5]);
        change(&here, &ids, Change::Delete { erase: true });
        assert_eq!(listed(&here, here.inbox), []);
        let pushed = push(&here, steps(vec![caps("UIDPLUS"), refusal])).await.unwrap();
        assert_eq!(pushed, [Dropped::Refused(words.into())]);
        assert_eq!(waiting(&here), 0);
        assert_eq!(listed(&here, here.inbox), ids, "{words}");
        assert!(here.store.lock().unwrap().original(ids[0]).unwrap().is_some());
    }
}

#[tokio::test]
async fn a_refused_folder_undoes_a_flag() {
    let marks = [(Change::Seen(true), "\\Seen"), (Change::Flagged(true), "\\Flagged")];
    for (mark, flag) in marks {
        for refusal in [refused_select(), refused_check()] {
            let (here, ids) = here(&[5]);
            change(&here, &ids, mark);
            assert!(flags(&here, 5).contains(&flag.to_owned()));
            let pushed = push(&here, steps(vec![caps(""), refusal])).await.unwrap();
            assert_eq!(pushed.len(), 1);
            assert_eq!(waiting(&here), 0);
            assert!(!flags(&here, 5).contains(&flag.to_owned()), "{flag}");
        }
    }
}

#[tokio::test]
async fn a_refused_folder_undoes_a_move() {
    for (refusal, words) in [(refused_select(), "NO [UNAVAILABLE] try later"), (refused_check(), "NO busy")] {
        let (here, ids) = here(&[5]);
        change(&here, &ids, Change::Move(here.work));
        assert_eq!(listed(&here, here.work), ids);
        let pushed = push(&here, steps(vec![caps("MOVE"), refusal])).await.unwrap();
        assert_eq!(pushed, [Dropped::Refused(words.into())]);
        assert_eq!(waiting(&here), 0);
        assert_eq!(listed(&here, here.inbox), ids, "{words}");
        assert_eq!(listed(&here, here.work), [], "{words}");
    }
}

#[test]
fn a_source_no_longer_listed_tells_the_move() {
    let (here, ids) = here(&[1]);
    change(&here, &ids, Change::Move(here.work));
    let mut store = here.store.lock().unwrap();
    store.set_folders(here.id, &[("Work", None)]).unwrap();
    assert_eq!(store.synced(here.id).unwrap().waiting, 0);
    assert_eq!(store.take_dropped(here.id).unwrap(), [Dropped::Vanished("INBOX".into())]);
}
