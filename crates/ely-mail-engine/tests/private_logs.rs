//! The real logger at trace, around real protocol traffic.

use base64::{Engine, engine::general_purpose::STANDARD};
use ely_mail_engine::{Imap, Pop, Secret, Security, Smtp};
use ely_platform::AppId;
use fake::{Step, certs, expect, login, send, serve};

const PASSWORD: &str = "PRIVATE_PASSWORD";
const TOKEN: &str = "PRIVATE_TOKEN";
const BODY: &str = "PRIVATE_BODY";

#[tokio::test]
async fn trace_logs_keep_no_password_token_or_mail() {
    let dir = tempfile::tempdir().unwrap();
    // The only test here; no thread reads env.
    unsafe { std::env::set_var("ELY_LOG", "trace") };
    ely_platform::logging::init_in(AppId::Mail, dir.path()).unwrap();
    let certs = certs();
    let message = format!("Subject: x\n\n{BODY}\n");

    let (server, script) = serve(
        &certs,
        Security::Tls,
        vec![
            send("* OK ready\n"),
            expect(&format!("{{tag}} LOGIN \"ann@example.com\" \"{PASSWORD}\"")),
            send("{tag} OK in\n"),
            expect("{tag} CAPABILITY"),
            send("* CAPABILITY IMAP4rev1\n{tag} OK caps\n"),
            expect("{tag} EXAMINE \"INBOX\""),
            send("* 1 EXISTS\n* OK [UIDVALIDITY 7] v\n{tag} OK done\n"),
            expect("{tag} UID FETCH 7 (UID FLAGS BODY.PEEK[])"),
            send(&format!(
                "* 1 FETCH (UID 7 BODY[] {{{}}}\n{message})\n{{tag}} OK fetched\n",
                message.replace('\n', "\r\n").len()
            )),
        ],
    )
    .await;
    let mut imap = Imap::connect(
        &server,
        &login(Secret::Password(PASSWORD.into())),
        &certs.trusted,
    )
    .await
    .unwrap();
    assert!(imap.fetch("INBOX", 7).await.unwrap().is_some());
    script.await.unwrap().unwrap();

    let (server, script) = serve(
        &certs,
        Security::Tls,
        vec![
            send("+OK ready\n"),
            expect("USER ann@example.com"),
            send("+OK\n"),
            expect(&format!("PASS {PASSWORD}")),
            send("+OK in\n"),
            expect("RETR 1"),
            send(&format!("+OK\n{message}.\n")),
        ],
    )
    .await;
    let mut pop = Pop::connect(
        &server,
        &login(Secret::Password(PASSWORD.into())),
        &certs.trusted,
    )
    .await
    .unwrap();
    pop.fetch(1).await.unwrap();
    script.await.unwrap().unwrap();

    let initial = STANDARD.encode(format!(
        "user=ann@example.com\x01auth=Bearer {TOKEN}\x01\x01"
    ));
    let (server, script) = serve(
        &certs,
        Security::Tls,
        vec![
            send("220 mx ready\n"),
            expect("EHLO [127.0.0.1]"),
            send("250-mx\n250 AUTH XOAUTH2\n"),
            expect(&format!("AUTH XOAUTH2 {initial}")),
            send("235 ok\n"),
            expect("MAIL FROM:<ann@example.com>"),
            send("250 ok\n"),
            expect("RCPT TO:<bo@example.com>"),
            send("250 ok\n"),
            expect("DATA"),
            send("354 go\n"),
            Step::Data(format!("Subject: x\n\n{BODY}")),
            send("250 queued\n"),
        ],
    )
    .await;
    let mut smtp = Smtp::connect(&server, &login(Secret::Token(TOKEN.into())), &certs.trusted)
        .await
        .unwrap();
    smtp.send(
        "ann@example.com",
        &["bo@example.com".to_owned()],
        message.replace('\n', "\r\n").as_bytes(),
    )
    .await
    .unwrap();
    script.await.unwrap().unwrap();

    log::logger().flush();
    let logged = std::fs::read_to_string(dir.path().join("ely.log")).unwrap();
    for line in ["IMAP login", "POP3 login", "SMTP login"] {
        assert!(logged.contains(line), "missing {line:?} in\n{logged}");
    }
    for secret in [PASSWORD, TOKEN, BODY, &initial] {
        assert!(
            !logged.contains(secret),
            "{secret} reached the log:\n{logged}"
        );
    }
}
