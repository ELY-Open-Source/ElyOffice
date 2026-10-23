use std::time::Duration;

use base64::{Engine, engine::general_purpose::STANDARD};
use ely_mail_engine::{Error, Secret, Security};
use fake::{Step, account, certs, expect, send, serve, smtp::logged_in};

const EHLO: &str = "250-mx\n250 AUTH PLAIN\n";

fn handed() -> Vec<Step> {
    vec![
        expect("MAIL FROM:<ann@example.com>"),
        send("250 ok\n"),
        expect("RCPT TO:<bo@example.com>"),
        send("250 ok\n"),
        expect("DATA"),
        send("354 go\n"),
        Step::Data("Subject: x\n\nx".into()),
    ]
}

#[tokio::test]
async fn a_lost_answer_after_the_message_is_unsure() {
    let (mut smtp, script) = logged_in(EHLO, handed()).await;
    let err = smtp
        .send(
            "ann@example.com",
            &["bo@example.com".to_owned()],
            b"Subject: x\r\n\r\nx\r\n",
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Unsure(_)), "{err}");
    script.await.unwrap().unwrap();
}

#[tokio::test]
async fn a_refusal_after_the_message_is_a_failure() {
    let mut steps = handed();
    steps.extend([send("554 5.7.1 Message rejected\n")]);
    let (mut smtp, script) = logged_in(EHLO, steps).await;
    let err = smtp
        .send(
            "ann@example.com",
            &["bo@example.com".to_owned()],
            b"Subject: x\r\n\r\nx\r\n",
        )
        .await
        .unwrap_err();
    assert!(
        matches!(&err, Error::Server(text) if text.contains("554") && text.contains("rejected")),
        "{err}"
    );
    script.await.unwrap().unwrap();
}

#[tokio::test]
async fn an_account_sends_to_every_envelope_address_and_ignores_a_failed_logout() {
    let certs = certs();
    let (server, script) = serve(
        &certs,
        Security::Tls,
        vec![
            send("220 mx ready\n"),
            expect("EHLO [127.0.0.1]"),
            send("250-mx\n250-SIZE 1000\n250 AUTH PLAIN\n"),
            expect(&format!(
                "AUTH PLAIN {}",
                STANDARD.encode("\0ann@example.com\0pw")
            )),
            send("235 ok\n"),
            expect("MAIL FROM:<ann@example.com>"),
            send("250 ok\n"),
            expect("RCPT TO:<bo@example.com>"),
            send("250 ok\n"),
            expect("RCPT TO:<hidden@example.com>"),
            send("250 ok\n"),
            expect("DATA"),
            send("354 go\n"),
            Step::Data("Subject: x\n\nx".into()),
            send("250 queued\n"),
            expect("QUIT"),
        ],
    )
    .await;
    let mut sender = account("ann@example.com");
    sender.outgoing = server;
    let limit = sender
        .send(
            Secret::Password("pw".into()),
            &["bo@example.com".to_owned(), "hidden@example.com".to_owned()],
            b"Subject: x\r\n\r\nx\r\n",
            &certs.trusted,
        )
        .await
        .unwrap();
    assert_eq!(limit, Some(1000), "the server's SIZE");
    script.await.unwrap().unwrap();
}

#[tokio::test]
async fn accepted_is_told_without_waiting_for_the_logout() {
    let certs = certs();
    let (server, script) = serve(
        &certs,
        Security::Tls,
        vec![
            send("220 mx ready\n"),
            expect("EHLO [127.0.0.1]"),
            send("250-mx\n250 AUTH PLAIN\n"),
            expect(&format!(
                "AUTH PLAIN {}",
                STANDARD.encode("\0ann@example.com\0pw")
            )),
            send("235 ok\n"),
            expect("MAIL FROM:<ann@example.com>"),
            send("250 ok\n"),
            expect("RCPT TO:<bo@example.com>"),
            send("250 ok\n"),
            expect("DATA"),
            send("354 go\n"),
            Step::Data("Subject: x\n\nx".into()),
            send("250 queued\n"),
            expect("QUIT"),
            Step::Hang,
        ],
    )
    .await;
    let mut sender = account("ann@example.com");
    sender.outgoing = server;
    let to = ["bo@example.com".to_owned()];
    let sent = sender.send(
        Secret::Password("pw".into()),
        &to,
        b"Subject: x\r\n\r\nx\r\n",
        &certs.trusted,
    );
    let limit = tokio::time::timeout(Duration::from_secs(5), sent)
        .await
        .expect("told before the logout answers")
        .unwrap();
    assert_eq!(limit, None);
    tokio::time::timeout(Duration::from_secs(5), script)
        .await
        .expect("the server reached its last step")
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn a_refusal_is_told_without_waiting_for_the_server() {
    let certs = certs();
    let (server, script) = serve(
        &certs,
        Security::Tls,
        vec![
            send("220 mx ready\n"),
            expect("EHLO [127.0.0.1]"),
            send("250-mx\n250 AUTH PLAIN\n"),
            expect(&format!(
                "AUTH PLAIN {}",
                STANDARD.encode("\0ann@example.com\0pw")
            )),
            send("235 ok\n"),
            expect("MAIL FROM:<ann@example.com>"),
            send("250 ok\n"),
            expect("RCPT TO:<bo@example.com>"),
            send("250 ok\n"),
            expect("DATA"),
            send("354 go\n"),
            Step::Data("Subject: x\n\nx".into()),
            send("554 5.7.1 Message rejected\n"),
            Step::Hang,
        ],
    )
    .await;
    let mut sender = account("ann@example.com");
    sender.outgoing = server;
    let to = ["bo@example.com".to_owned()];
    let sent = sender.send(
        Secret::Password("pw".into()),
        &to,
        b"Subject: x\r\n\r\nx\r\n",
        &certs.trusted,
    );
    let err = tokio::time::timeout(Duration::from_secs(5), sent)
        .await
        .expect("told before the server answers again")
        .unwrap_err();
    assert!(
        matches!(&err, Error::Server(text) if text.contains("554") && text.contains("rejected")),
        "{err}"
    );
    tokio::time::timeout(Duration::from_secs(5), script)
        .await
        .expect("the server reached its last step")
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn a_message_over_the_size_limit_never_starts() {
    let (mut smtp, script) = logged_in("250-mx\n250-SIZE 20\n250 AUTH PLAIN\n", vec![]).await;
    assert_eq!(smtp.limit(), Some(20));
    let message = b"Subject: x\r\n\r\nmore than twenty bytes\r\n";
    let err = smtp
        .send("ann@example.com", &["bo@example.com".to_owned()], message)
        .await
        .unwrap_err();
    let size = message.len() as u64;
    assert!(
        matches!(err, Error::TooLarge { size: s, limit: 20 } if s == size),
        "{err}"
    );
    drop(smtp);
    script.await.unwrap().unwrap();
}

#[tokio::test]
async fn size_zero_is_no_limit() {
    let (smtp, script) = logged_in("250-mx\n250-SIZE 0\n250 AUTH PLAIN\n", vec![]).await;
    assert_eq!(smtp.limit(), None);
    drop(smtp);
    script.await.unwrap().unwrap();
}

#[test]
fn the_big_providers_file_sent_mail_themselves() {
    let mut sender = account("ann@example.com");
    assert!(!sender.files_sent_itself(), "a plain server does not");
    sender.outgoing.host = "SMTP.mail.me.com".into();
    assert!(sender.files_sent_itself(), "iCloud");
    sender.outgoing.host = "smtp.example.com".into();
    sender.auth = ely_mail_engine::Auth::Microsoft;
    assert!(sender.files_sent_itself(), "Microsoft by OAuth");
    sender.auth = ely_mail_engine::Auth::Password;
    sender.outgoing.host = "smtp.gmail.com".into();
    assert!(sender.files_sent_itself(), "Gmail by app password");
}
