//! The whole window with no server reachable (C12.23, QA-16).

use std::time::Duration;

use ely_gpui_component::documents::MarkdownBlock;
use ely_mail_engine::{Fetched, Going, Key, MessageId, Role, Store};
use gpui::{Entity, TestAppContext, VisualTestContext};

use super::{
    account, closed_port,
    drafts::{new_message, pause, with, write},
    leaving::kept,
    outbox::{outbox, with_password},
    panes::shows,
    reading::opened,
    settle, window,
    writing::draft,
};
use crate::mail::Mail;

const HEAD: &str = "From: Ann <ann@example.com>\r\nTo: bo@example.com\r\nDate: 1 Jan 2024 09:00:00 +0000\r\n";

const FILED: &str = "Content-Type: multipart/mixed; boundary=b\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nsee the budget\r\n\
--b\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment; filename=plan.pdf\r\n\r\n%PDF-1.7 plan\r\n--b--\r\n";

/// Bo offline: one whole message, one without its file.
fn seeded(dir: &std::path::Path) -> (MessageId, MessageId) {
    let mut store = Store::open(dir).unwrap();
    let mut bo = account("bo@example.com");
    bo.incoming.port = closed_port();
    bo.outgoing.port = closed_port();
    let id = store.add_account(&bo).unwrap();
    let [inbox] = store.set_folders(id, &[("INBOX", Some(Role::Inbox))]).unwrap().try_into().unwrap();
    let key = |uid| Key::Imap { uidvalidity: 1, uid };
    let whole = format!("{HEAD}Subject: plan\r\nMessage-ID: <w@x>\r\n{FILED}");
    let whole = store.put(inbox, &key(1), &["\\Seen"], Fetched::Whole(whole.as_bytes())).unwrap();
    let cut = FILED.replace("%PDF-1.7 plan\r\n", "").replace("budget", "notes");
    let cut = format!("{HEAD}Subject: notes\r\nMessage-ID: <c@x>\r\n{cut}");
    let missing = ["2".to_owned()];
    let fetched = Fetched::Text { bytes: cut.as_bytes(), missing: &missing };
    let partial = store.put(inbox, &key(2), &["\\Seen"], fetched).unwrap();
    (whole, partial)
}

fn shown(mail: &Entity<Mail>, id: MessageId, cx: &mut VisualTestContext) -> Vec<(String, Option<u64>)> {
    let key = id.0.to_string().into();
    cx.update(|_, app| mail.update(app, |mail, cx| mail.select(&[key], cx)));
    settle(cx, |app| mail.read(app).reading.files(id).is_some_and(|files| !files.is_empty()));
    cx.update(|_, app| mail.read(app).reading.files(id).unwrap().to_vec())
}

#[test]
fn offline_mail_reads_searches_keeps_drafts_and_queues() {
    let dir = tempfile::tempdir().unwrap();
    let (whole, partial) = seeded(dir.path());
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), with_password());
    cx.update(|_, app| mail.update(app, |mail, cx| mail.sync_all(cx)));
    settle(cx, |app| mail.read(app).syncs.values().all(|each| each.task.is_none() && each.failed.is_some()));
    shows(&mail, cx, &["notes", "plan"]);
    assert_eq!(shown(&mail, whole, cx), [("plan.pdf".to_owned(), Some(13))]);
    let read = cx.update(|_, app| {
        let open = opened(&mail, app).unwrap();
        open.blocks.iter().any(|block| matches!(block, MarkdownBlock::Paragraph(line) if line.text == "see the budget"))
    });
    assert!(read, "the cached body reads");
    assert_eq!(shown(&mail, partial, cx), [("plan.pdf".to_owned(), None)]);
    let key = partial.0.to_string().into();
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_file(&key, 0, cx)));
    settle(cx, |app| mail.read(app).downloading.is_empty());
    assert_eq!(shown(&mail, partial, cx), [("plan.pdf".to_owned(), None)], "a failed fetch is not here");
    assert!(!dir.path().join("opened").exists(), "nothing opens unfetched");
    cx.update(|_, app| {
        let input = mail.read(app).seeking.input.clone();
        input.update(app, |input, cx| input.set_text("budget", cx));
    });
    cx.executor().advance_clock(Duration::from_millis(250));
    shows(&mail, cx, &["plan"]);
    let compose = new_message(&mail, cx);
    write(compose, "Lunch?", cx);
    let addressed = draft(&["ann@example.com"], "Hi");
    with(compose, cx, |compose, _, cx| compose.edited(addressed.clone(), cx));
    pause(&mail, cx, 1);
    assert_eq!(kept(&mail, cx)[0].unsent.words, "Lunch?");
    with(compose, cx, |compose, window, cx| compose.send(&addressed, window, cx));
    settle(cx, |app| {
        let waiting = outbox(&mail, app);
        waiting.len() == 1 && waiting[0].going == Going::Queued && waiting[0].error.is_some()
    });
    assert!(kept(&mail, cx).is_empty(), "the draft went to the Outbox");
}
