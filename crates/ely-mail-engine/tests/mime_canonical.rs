//! Composites go unencoded only when canonical; others as files.

use ely_mail_engine::{Draft, Mailbox, Part, build, compose, read};
use fake::{Step, expect, send, smtp::logged_in};

fn draft(attachment: Part) -> Draft {
    Draft {
        from: Mailbox {
            name: None,
            email: "ann@example.com".into(),
        },
        to: Vec::new(),
        cc: Vec::new(),
        bcc: Vec::new(),
        subject: "fwd".into(),
        text: "see attached".into(),
        html: None,
        attachments: vec![attachment],
        in_reply_to: None,
        references: Vec::new(),
        date: 0,
    }
}

/// Built, sent to a server without 8BITMIME, read back.
async fn delivered(attachment: Part) -> Part {
    let built = build(&draft(attachment)).unwrap().bytes;
    let steps = vec![
        expect("MAIL FROM:<ann@example.com>"),
        send("250 ok\n"),
        expect("RCPT TO:<bo@example.com>"),
        send("250 ok\n"),
        expect("DATA"),
        send("354 go\n"),
        Step::AnyData,
        send("250 queued\n"),
    ];
    let (mut smtp, script) = logged_in("250-mx\n250 AUTH PLAIN\n", steps).await;
    let to = ["bo@example.com".to_owned()];
    smtp.send("ann@example.com", &to, &built).await.unwrap();
    script.await.unwrap().unwrap();
    read(&built).unwrap().attachments.remove(0)
}

/// `file` arrives whole, under its name, as plain bytes.
async fn as_a_file(name: &str, file: &[u8]) {
    let part = compose::attachment(name, file.to_vec());
    let got = delivered(part).await;
    assert_eq!((got.content_type.as_str(), got.name.as_deref()), ("application/octet-stream", Some(name)));
    assert_eq!(got.bytes, file);
}

#[tokio::test]
async fn a_canonical_eml_stays_a_message() {
    let file = b"Subject: hi\r\nFrom: c@example.com\r\n\r\nhello\r\n";
    let got = delivered(compose::attachment("note.eml", file.to_vec())).await;
    assert_eq!(got.content_type, "message/rfc822");
    assert_eq!(got.bytes, file);
}

#[tokio::test]
async fn an_eml_with_bare_line_ends_goes_as_a_file() {
    as_a_file("note.eml", b"Subject: hi\nFrom: c@example.com\r\n\nhello\rbye\n").await;
}

#[tokio::test]
async fn binary_inside_an_eml_keeps_its_bytes() {
    let mut file = b"Content-Type: image/png\r\nContent-Transfer-Encoding: binary\r\n\r\n".to_vec();
    file.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x0D, 0x49, 0x0A]);
    as_a_file("pic.eml", &file).await;
}

#[tokio::test]
async fn a_nul_in_crlf_lines_goes_as_a_file() {
    as_a_file("nul.eml", b"Subject: hi\r\n\r\nhel\0lo\r\n").await;
}

#[tokio::test]
async fn an_eml_line_past_998_octets_goes_as_a_file() {
    let fits = format!("Subject: hi\r\n\r\n{}\r\n", "a".repeat(998));
    let got = delivered(compose::attachment("fits.eml", fits.clone().into_bytes())).await;
    assert_eq!(got.content_type, "message/rfc822");
    as_a_file("long.eml", format!("Subject: hi\r\n\r\n{}\r\n", "a".repeat(999)).as_bytes()).await;
}

#[tokio::test]
async fn a_multipart_with_lf_lines_goes_as_a_file() {
    let bytes = b"Content-Type: multipart/mixed; boundary=b\n\n--b\nContent-Type: text/plain\n\nx\n--b--\n".to_vec();
    let part = Part {
        name: Some("parts".into()),
        content_type: "multipart/mixed".into(),
        params: vec![("boundary".into(), "b".into())],
        content_id: None,
        bytes: bytes.clone(),
        section: None,
    };
    let got = delivered(part).await;
    assert_eq!(got.content_type, "application/octet-stream");
    assert_eq!(got.bytes, bytes);
}
