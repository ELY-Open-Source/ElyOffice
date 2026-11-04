use ely_mail_engine::{
    Error, Imap, Protocol, Remote, Secret, Security,
    drafts::{Placed, put},
};
use fake::{Step, account, certs, expect, login, send, serve};

const MESSAGE: &[u8] = b"Subject: x\r\n\r\nhi\r\n";

fn logged_in(caps: &str) -> Vec<Step> {
    vec![
        send("* OK ready\n"),
        expect("{tag} LOGIN \"ann@example.com\" \"pw\""),
        send("{tag} OK done\n"),
        expect("{tag} CAPABILITY"),
        send(&format!("* CAPABILITY {caps}\n{{tag}} OK caps\n")),
    ]
}

async fn run(steps: Vec<Step>, act: impl AsyncFnOnce(&mut Imap)) {
    let certs = certs();
    let (server, script) = serve(&certs, Security::Tls, steps).await;
    let secret = Secret::Password("pw".into());
    let mut imap = Imap::connect(&server, &login(secret), &certs.trusted)
        .await
        .unwrap();
    act(&mut imap).await;
    drop(imap);
    script.await.unwrap().unwrap();
}

fn old() -> Remote {
    Remote {
        mailbox: "Drafts".into(),
        uidvalidity: 9,
        uid: 31,
    }
}

#[tokio::test]
async fn a_draft_lands_where_the_server_says() {
    let mut steps = logged_in("IMAP4rev1 UIDPLUS");
    steps.extend(appended());
    run(steps, async |imap| {
        let placed = imap.put_draft("Drafts", MESSAGE).await.unwrap();
        let want = Remote { uid: 33, ..old() };
        assert_eq!(placed, want);
    })
    .await;
}

#[tokio::test]
async fn without_uidplus_nothing_is_saved() {
    run(logged_in("IMAP4rev1"), async |imap| {
        let err = imap.put_draft("Drafts", MESSAGE).await.unwrap_err();
        assert!(matches!(err, Error::Unsupported("UIDPLUS")), "{err}");
    })
    .await;
}

#[tokio::test]
async fn a_refused_draft_says_why() {
    let mut steps = logged_in("IMAP4rev1 UIDPLUS");
    steps.extend([
        expect("{tag} APPEND \"Drafts\" (\\Draft \\Seen) {18}"),
        send("{tag} NO [OVERQUOTA] full\n"),
    ]);
    run(steps, async |imap| {
        let err = imap.put_draft("Drafts", MESSAGE).await.unwrap_err();
        assert!(
            matches!(err, Error::Server(ref words) if words.contains("full")),
            "{err}"
        );
    })
    .await;
}

#[tokio::test]
async fn the_old_copy_goes_by_its_uid() {
    let mut steps = logged_in("IMAP4rev1 UIDPLUS");
    steps.extend([
        expect("{tag} SELECT \"Drafts\""),
        send("* 2 EXISTS\n* OK [UIDVALIDITY 9] v\n{tag} OK [READ-WRITE] in\n"),
        expect("{tag} UID STORE 31 +FLAGS.SILENT (\\Deleted)"),
        send("{tag} OK stored\n"),
        expect("{tag} UID EXPUNGE 31"),
        send("* 1 EXPUNGE\n{tag} OK gone\n"),
    ]);
    run(steps, async |imap| {
        assert!(imap.drop_draft(&old()).await.unwrap());
    })
    .await;
}

#[tokio::test]
async fn a_renumbered_folder_keeps_its_copy() {
    let mut steps = logged_in("IMAP4rev1 UIDPLUS");
    steps.extend([
        expect("{tag} SELECT \"Drafts\""),
        send("* 2 EXISTS\n* OK [UIDVALIDITY 10] v\n{tag} OK [READ-WRITE] in\n"),
    ]);
    run(steps, async |imap| {
        assert!(!imap.drop_draft(&old()).await.unwrap());
    })
    .await;
}

fn appended() -> [Step; 7] {
    [
        expect("{tag} APPEND \"Drafts\" (\\Draft \\Seen) {18}"),
        send("+ go\n"),
        expect("Subject: x"),
        expect(""),
        expect("hi"),
        expect(""),
        send("{tag} OK [APPENDUID 9 33] saved\n"),
    ]
}

async fn put_with(old_goes: Vec<Step>) -> Placed {
    let mut steps = logged_in("IMAP4rev1 UIDPLUS");
    steps.extend(appended());
    steps.push(expect("{tag} CAPABILITY"));
    steps.push(send("* CAPABILITY IMAP4rev1 UIDPLUS\n{tag} OK caps\n"));
    steps.extend(old_goes);
    steps.extend([expect("{tag} LOGOUT"), send("* BYE\n{tag} OK bye\n")]);
    let certs = certs();
    let (server, script) = serve(&certs, Security::Tls, steps).await;
    let mut ann = account("ann@example.com");
    ann.incoming = server;
    let secret = Secret::Password("pw".into());
    let placed = put(
        &ann,
        secret,
        &certs.trusted,
        "Drafts",
        MESSAGE,
        Some(&old()),
    )
    .await
    .unwrap();
    script.await.unwrap().unwrap();
    placed
}

#[tokio::test]
async fn a_new_copy_replaces_the_old() {
    let placed = put_with(vec![
        expect("{tag} SELECT \"Drafts\""),
        send("* 2 EXISTS\n* OK [UIDVALIDITY 9] v\n{tag} OK in\n"),
        expect("{tag} UID STORE 31 +FLAGS.SILENT (\\Deleted)"),
        send("{tag} OK stored\n"),
        expect("{tag} UID EXPUNGE 31"),
        send("{tag} OK gone\n"),
    ])
    .await;
    let want = Remote { uid: 33, ..old() };
    assert_eq!(
        placed,
        Placed {
            remote: want,
            left: None
        }
    );
}

#[tokio::test]
async fn an_old_copy_that_stays_says_why() {
    let placed = put_with(vec![
        expect("{tag} SELECT \"Drafts\""),
        send("{tag} NO [NONEXISTENT] no such folder\n"),
    ])
    .await;
    assert_eq!(placed.remote.uid, 33);
    let left = placed.left.expect("the old copy stayed");
    assert!(left.contains("no such folder"), "{left}");
}

#[tokio::test]
async fn pop_keeps_drafts_here() {
    let mut ann = account("ann@example.com");
    ann.protocol = Protocol::Pop { keep: true };
    let secret = Secret::Password("pw".into());
    let err = put(&ann, secret, &certs().trusted, "Drafts", MESSAGE, None)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Invalid(_)), "{err}");
}

#[tokio::test]
async fn a_sent_copy_is_filed_read_and_needs_no_uidplus() {
    let mut steps = logged_in("IMAP4rev1");
    steps.truncate(3);
    steps.extend([
        expect("{tag} APPEND \"Sent\" (\\Seen) {18}"),
        send("+ go\n"),
        expect("Subject: x"),
        expect(""),
        expect("hi"),
        expect(""),
        send("{tag} OK saved\n"),
        expect("{tag} LOGOUT"),
        send("* BYE\n{tag} OK bye\n"),
    ]);
    let certs = certs();
    let (server, script) = serve(&certs, Security::Tls, steps).await;
    let mut ann = account("ann@example.com");
    ann.incoming = server;
    let secret = Secret::Password("pw".into());
    let done = ely_mail_engine::sent::file(&ann, secret, &certs.trusted, "Sent", MESSAGE).await;
    done.unwrap();
    script.await.unwrap().unwrap();
}

#[tokio::test]
async fn a_filed_copy_is_told_without_waiting_for_the_logout() {
    let mut steps = logged_in("IMAP4rev1");
    steps.truncate(3);
    steps.extend([
        expect("{tag} APPEND \"Sent\" (\\Seen) {18}"),
        send("+ go\n"),
        expect("Subject: x"),
        expect(""),
        expect("hi"),
        expect(""),
        send("{tag} OK saved\n"),
        expect("{tag} LOGOUT"),
        Step::Hang,
    ]);
    let certs = certs();
    let (server, script) = serve(&certs, Security::Tls, steps).await;
    let mut ann = account("ann@example.com");
    ann.incoming = server;
    let secret = Secret::Password("pw".into());
    let done = ely_mail_engine::sent::file(&ann, secret, &certs.trusted, "Sent", MESSAGE);
    tokio::time::timeout(std::time::Duration::from_secs(5), done)
        .await
        .expect("told before the logout answers")
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(60), script)
        .await
        .expect("the server reached its last step")
        .unwrap()
        .unwrap();
}
