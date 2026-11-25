//! A v6 store links the messages it already holds.

use super::{STEPS, held, open};
use crate::{MessageId, Role};

#[test]
fn a_v6_store_links_its_messages_on_upgrade() {
    let dir = tempfile::tempdir().unwrap();
    let old = open(dir.path(), &STEPS[..6]).unwrap();
    let ann = super::old_account(&old, "ann@example.com");
    let [inbox] = super::old_folders(&old, ann, &[("INBOX", Some(Role::Inbox))])
        .try_into()
        .unwrap();
    let mut ids = Vec::new();
    // Id, In-Reply-To, References; the third has no Message-ID.
    for (uid, (id, reply, refs)) in [
        (Some("a@x"), "[]", "[]"),
        (Some("b@x"), r#"["a@x"]"#, "[]"),
        (None, "[]", r#"["a@x","b@x"]"#),
        (Some("lone@x"), "[]", r#"[""]"#),
    ]
    .into_iter()
    .enumerate()
    {
        old.conn
            .execute(
                "INSERT INTO messages (mailbox, uidvalidity, uid, message_id, in_reply_to, refs,
                     sender, recipients, copies, content, date)
                 VALUES (?1, 1, ?2, ?3, ?4, ?5, '[]', '[]', '[]', ?6, ?2)",
                rusqlite::params![inbox.0, uid as i64, id, reply, refs, held(dir.path())],
            )
            .unwrap();
        ids.push(MessageId(old.conn.last_insert_rowid()));
    }
    drop(old);
    let store = open(dir.path(), STEPS).unwrap();
    assert_eq!(store.read_only(), None);
    let thread = |id| -> Vec<MessageId> {
        let found = store.thread(id).unwrap();
        found.into_iter().map(|each| each.id).collect()
    };
    assert_eq!(thread(ids[2]), ids[..3]);
    assert_eq!(thread(ids[0]), ids[..3]);
    assert_eq!(thread(ids[3]), [ids[3]], "an empty reference links nothing");
}
