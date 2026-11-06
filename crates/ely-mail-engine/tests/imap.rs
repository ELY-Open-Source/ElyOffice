use base64::{Engine, engine::general_purpose::STANDARD};
use ely_mail_engine::{Error, Imap, Role, Secret, Security, system_tls};
use fake::{certs, expect, login, send, serve};
use tokio::task::JoinHandle;

const TOKEN_LINE: &str = "user=ann@example.com\x01auth=Bearer tok-1\x01\x01";

#[tokio::test]
async fn a_password_login_lists_and_fetches() {
    let certs = certs();
    let lines = "Subject: Hi\n\nHello\n";
    let body = lines.replace('\n', "\r\n");
    let (server, script) = serve(
        &certs,
        Security::Tls,
        vec![
            send("* OK ready\n"),
            expect("{tag} LOGIN \"ann@example.com\" \"pw 1\""),
            send("{tag} OK done\n"),
            expect("{tag} CAPABILITY"),
            send("* CAPABILITY IMAP4rev1\n{tag} OK caps\n"),
            expect("{tag} LIST \"\" *"),
            send("* LIST (\\HasNoChildren) \"/\" INBOX\n"),
            send("* LIST (\\HasNoChildren \\Sent) \"/\" \"Sent Mail\"\n"),
            send("{tag} OK listed\n"),
            expect("{tag} EXAMINE \"INBOX\""),
            send("* 1 EXISTS\n* OK [UIDVALIDITY 7] v\n{tag} OK [READ-ONLY] in\n"),
            expect("{tag} UID FETCH 42 (UID FLAGS BODY.PEEK[])"),
            send(&format!(
                "* 1 FETCH (UID 42 BODY[] {{{}}}\n{lines})\n{{tag}} OK fetched\n",
                body.len()
            )),
            expect("{tag} LOGOUT"),
            send("* BYE\n{tag} OK bye\n"),
        ],
    )
    .await;
    let mut imap = Imap::connect(
        &server,
        &login(Secret::Password("pw 1".into())),
        &certs.trusted,
    )
    .await
    .unwrap();
    let folders = imap.folders().await.unwrap();
    let names: Vec<_> = folders.iter().map(|f| (f.name.as_str(), f.role)).collect();
    assert_eq!(
        names,
        [
            ("INBOX", Some(Role::Inbox)),
            ("Sent Mail", Some(Role::Sent))
        ]
    );
    let fetched = imap.fetch("INBOX", 42).await.unwrap().unwrap();
    assert_eq!(String::from_utf8(fetched).unwrap(), body);
    imap.logout().await.unwrap();
    script.await.unwrap().unwrap();
}

#[tokio::test]
async fn starttls_then_xoauth2() {
    let certs = certs();
    let (server, script) = serve(
        &certs,
        Security::StartTls,
        vec![
            send("* OK ready\n"),
            expect("{tag} STARTTLS"),
            send("{tag} OK go\n"),
            fake::Step::StartTls,
            expect("{tag} AUTHENTICATE XOAUTH2"),
            send("+ \n"),
            expect(&STANDARD.encode(TOKEN_LINE)),
            send("{tag} OK welcome\n"),
            expect("{tag} CAPABILITY"),
            send("* CAPABILITY IMAP4rev1\n{tag} OK caps\n"),
            expect("{tag} LOGOUT"),
            send("* BYE\n{tag} OK bye\n"),
        ],
    )
    .await;
    let imap = Imap::connect(
        &server,
        &login(Secret::Token("tok-1".into())),
        &certs.trusted,
    )
    .await
    .unwrap();
    imap.logout().await.unwrap();
    script.await.unwrap().unwrap();
}

#[tokio::test]
async fn a_refused_password_keeps_the_server_words() {
    let certs = certs();
    let (server, script) = serve(
        &certs,
        Security::Tls,
        vec![
            send("* OK ready\n"),
            expect("{tag} LOGIN \"ann@example.com\" \"bad\""),
            send("{tag} NO [AUTHENTICATIONFAILED] Invalid credentials\n"),
        ],
    )
    .await;
    let err = Imap::connect(
        &server,
        &login(Secret::Password("bad".into())),
        &certs.trusted,
    )
    .await
    .err()
    .unwrap();
    assert!(
        matches!(&err, Error::Rejected(text) if text.contains("Invalid credentials")),
        "{err}"
    );
    script.await.unwrap().unwrap();
}

#[tokio::test]
async fn a_refused_token_answers_the_error_challenge_empty() {
    let certs = certs();
    let (server, script) = serve(
        &certs,
        Security::Tls,
        vec![
            send("* OK ready\n"),
            expect("{tag} AUTHENTICATE XOAUTH2"),
            send("+ \n"),
            expect(&STANDARD.encode(TOKEN_LINE)),
            send(&format!("+ {}\n", STANDARD.encode(r#"{"status":"401"}"#))),
            expect(""),
            send("{tag} NO [AUTHENTICATIONFAILED] Invalid token\n"),
        ],
    )
    .await;
    let err = Imap::connect(
        &server,
        &login(Secret::Token("tok-1".into())),
        &certs.trusted,
    )
    .await
    .err()
    .unwrap();
    assert!(
        matches!(&err, Error::Rejected(text) if text.contains("Invalid token")),
        "{err}"
    );
    script.await.unwrap().unwrap();
}

#[tokio::test]
async fn a_missing_uid_is_none() {
    let certs = certs();
    let (server, script) = serve(
        &certs,
        Security::Tls,
        vec![
            send("* OK ready\n"),
            expect("{tag} LOGIN \"ann@example.com\" \"pw\""),
            send("{tag} OK done\n"),
            expect("{tag} CAPABILITY"),
            send("* CAPABILITY IMAP4rev1\n{tag} OK caps\n"),
            expect("{tag} EXAMINE \"INBOX\""),
            send("* 1 EXISTS\n* OK [UIDVALIDITY 7] v\n{tag} OK in\n"),
            expect("{tag} UID FETCH 9 (UID FLAGS BODY.PEEK[])"),
            send("{tag} OK nothing\n"),
        ],
    )
    .await;
    let mut imap = Imap::connect(
        &server,
        &login(Secret::Password("pw".into())),
        &certs.trusted,
    )
    .await
    .unwrap();
    assert_eq!(imap.fetch("INBOX", 9).await.unwrap(), None);
    script.await.unwrap().unwrap();
}

#[tokio::test]
async fn an_unreadable_reply_is_a_protocol_error() {
    let certs = certs();
    let (server, script) = serve(
        &certs,
        Security::Tls,
        vec![
            send("* OK ready\n"),
            expect("{tag} LOGIN \"ann@example.com\" \"pw\""),
            send("{tag} OK done\n"),
            expect("{tag} CAPABILITY"),
            send("* CAPABILITY IMAP4rev1\n{tag} OK caps\n"),
            expect("{tag} LIST \"\" *"),
            send("* LIST (((\n"),
        ],
    )
    .await;
    let mut imap = Imap::connect(
        &server,
        &login(Secret::Password("pw".into())),
        &certs.trusted,
    )
    .await
    .unwrap();
    let err = imap.folders().await.unwrap_err();
    assert!(matches!(err, Error::Protocol(_)), "{err}");
    script.await.unwrap().unwrap();
}

#[tokio::test]
async fn the_system_store_refuses_an_unknown_certificate() {
    let certs = certs();
    let (server, _script) = serve(&certs, Security::Tls, vec![send("* OK ready\n")]).await;
    let err = Imap::connect(
        &server,
        &login(Secret::Password("pw".into())),
        &system_tls().unwrap(),
    )
    .await
    .err()
    .unwrap();
    assert!(matches!(err, Error::Tls(_)), "{err}");
}

#[tokio::test]
async fn a_greeting_bye_keeps_the_server_words() {
    let certs = certs();
    let (server, script) = serve(
        &certs,
        Security::Tls,
        vec![send("* BYE [UNAVAILABLE] Maintenance in progress\n")],
    )
    .await;
    let err = Imap::connect(
        &server,
        &login(Secret::Password("pw".into())),
        &certs.trusted,
    )
    .await
    .err()
    .unwrap();
    assert!(
        matches!(&err, Error::Server(text) if text.contains("Maintenance in progress")),
        "{err}"
    );
    script.await.unwrap().unwrap();
}

/// Greeting and LOGIN "pw", then `steps`.
async fn logged_in(steps: Vec<fake::Step>) -> (Imap, JoinHandle<Result<(), String>>) {
    let certs = certs();
    let mut script = vec![
        send("* OK ready\n"),
        expect("{tag} LOGIN \"ann@example.com\" \"pw\""),
        send("{tag} OK done\n"),
        expect("{tag} CAPABILITY"),
        send("* CAPABILITY IMAP4rev1\n{tag} OK caps\n"),
    ];
    script.extend(steps);
    let (server, script) = serve(&certs, Security::Tls, script).await;
    let imap = Imap::connect(
        &server,
        &login(Secret::Password("pw".into())),
        &certs.trusted,
    )
    .await
    .unwrap();
    (imap, script)
}

#[tokio::test]
async fn listed_names_lose_quote_escapes_and_open_as_listed() {
    let (mut imap, script) = logged_in(vec![
        expect("{tag} LIST \"\" *"),
        send("* LIST () \"/\" \"a\\\"b\"\n"),
        send("* LIST () \"/\" \"c\\\\d\"\n"),
        send("* LIST () \"/\" {3}\ne\\\"\n"),
        send("{tag} OK listed\n"),
        expect("{tag} EXAMINE \"a\\\"b\""),
        send("* 1 EXISTS\n* OK [UIDVALIDITY 7] v\n{tag} OK in\n"),
        expect("{tag} UID FETCH 9 (UID FLAGS BODY.PEEK[])"),
        send("{tag} OK nothing\n"),
    ])
    .await;
    let names: Vec<_> = imap
        .folders()
        .await
        .unwrap()
        .into_iter()
        .map(|f| f.name)
        .collect();
    assert_eq!(names, ["a\"b", "c\\d", "e\\\""]);
    assert_eq!(imap.fetch(&names[0], 9).await.unwrap(), None);
    script.await.unwrap().unwrap();
}

#[tokio::test]
async fn a_refused_list_keeps_the_server_words() {
    let (mut imap, script) = logged_in(vec![
        expect("{tag} LIST \"\" *"),
        send("* LIST () \"/\" INBOX\n"),
        send("{tag} NO [UNAVAILABLE] Mailbox listing temporarily unavailable\n"),
    ])
    .await;
    let err = imap.folders().await.unwrap_err();
    assert!(
        matches!(&err, Error::Server(text) if text.contains("listing temporarily unavailable")),
        "{err}"
    );
    script.await.unwrap().unwrap();
}

#[tokio::test]
async fn a_refused_fetch_is_not_a_missing_uid() {
    let (mut imap, script) = logged_in(vec![
        expect("{tag} EXAMINE \"INBOX\""),
        send("* 1 EXISTS\n* OK [UIDVALIDITY 7] v\n{tag} OK in\n"),
        expect("{tag} UID FETCH 9 (UID FLAGS BODY.PEEK[])"),
        send("{tag} NO [UNAVAILABLE] Message store temporarily unavailable\n"),
    ])
    .await;
    let err = imap.fetch("INBOX", 9).await.unwrap_err();
    assert!(
        matches!(&err, Error::Server(text) if text.contains("store temporarily unavailable")),
        "{err}"
    );
    script.await.unwrap().unwrap();
}

#[tokio::test]
async fn a_bye_after_login_keeps_the_server_words() {
    let certs = certs();
    let (server, script) = serve(
        &certs,
        Security::Tls,
        vec![
            send("* OK ready\n"),
            expect("{tag} LOGIN \"ann@example.com\" \"pw\""),
            send("* BYE [UNAVAILABLE] Too many simultaneous connections\n"),
        ],
    )
    .await;
    let err = Imap::connect(
        &server,
        &login(Secret::Password("pw".into())),
        &certs.trusted,
    )
    .await
    .err()
    .unwrap();
    assert!(
        matches!(&err, Error::Server(text) if text.contains("Too many simultaneous connections")),
        "{err}"
    );
    script.await.unwrap().unwrap();

    let (mut imap, script) = logged_in(vec![
        expect("{tag} LIST \"\" *"),
        send("* BYE Shutting down\n"),
    ])
    .await;
    let err = imap.folders().await.unwrap_err();
    assert!(
        matches!(&err, Error::Server(text) if text.contains("Shutting down")),
        "{err}"
    );
    script.await.unwrap().unwrap();
}

#[tokio::test]
async fn a_password_beyond_ascii_goes_as_a_literal() {
    let certs = certs();
    let password = "pässwört";
    let (server, script) = serve(
        &certs,
        Security::Tls,
        vec![
            send("* OK ready\n"),
            expect(&format!(
                "{{tag}} LOGIN \"ann@example.com\" {{{}}}",
                password.len()
            )),
            send("+ go on\n"),
            expect(password),
            send("{tag} OK done\n"),
            expect("{tag} CAPABILITY"),
            send("* CAPABILITY IMAP4rev1\n{tag} OK caps\n"),
        ],
    )
    .await;
    Imap::connect(
        &server,
        &login(Secret::Password(password.into())),
        &certs.trusted,
    )
    .await
    .unwrap();
    script.await.unwrap().unwrap();
}

#[tokio::test]
async fn bytes_before_tls_are_refused() {
    let certs = certs();
    let (server, _script) = serve(
        &certs,
        Security::StartTls,
        vec![
            send("* OK ready\n"),
            expect("{tag} STARTTLS"),
            send("{tag} OK go\n* OK injected\n"),
        ],
    )
    .await;
    let err = Imap::connect(
        &server,
        &login(Secret::Password("pw".into())),
        &certs.trusted,
    )
    .await
    .err()
    .unwrap();
    assert!(
        matches!(&err, Error::Protocol(text) if text.contains("before TLS")),
        "{err}"
    );
}

#[tokio::test]
async fn a_server_offering_id_hears_the_client_name() {
    let certs = certs();
    let id = format!("{{tag}} ID (\"name\" \"ElyMail\" \"version\" \"{}\")", env!("CARGO_PKG_VERSION"));
    let (server, script) = serve(
        &certs,
        Security::Tls,
        vec![
            send("* OK ready\n"),
            expect("{tag} LOGIN \"ann@example.com\" \"pw\""),
            send("{tag} OK done\n"),
            expect("{tag} CAPABILITY"),
            send("* CAPABILITY IMAP4rev1 ID\n{tag} OK caps\n"),
            expect(&id),
            send("* ID (\"name\" \"Coremail\")\n{tag} OK ID completed\n"),
            expect("{tag} LOGOUT"),
            send("* BYE\n{tag} OK bye\n"),
        ],
    )
    .await;
    let imap = Imap::connect(&server, &login(Secret::Password("pw".into())), &certs.trusted)
        .await
        .unwrap();
    imap.logout().await.unwrap();
    script.await.unwrap().unwrap();
}
