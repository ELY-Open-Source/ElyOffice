//! Composite attachments go out with CRLF lines (RFC 2049 §4).

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

/// A plain server without 8BITMIME takes the message.
async fn sent(message: &[u8]) {
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
    smtp.send("ann@example.com", &to, message).await.unwrap();
    script.await.unwrap().unwrap();
}

#[tokio::test]
async fn an_eml_with_bare_line_ends_goes_out_in_crlf() {
    let file = b"Subject: hi\nFrom: c@example.com\r\n\nhello\rbye\n".to_vec();
    let part = compose::attachment("note.eml", file);
    assert_eq!(part.content_type, "message/rfc822");
    let built = build(&draft(part)).unwrap().bytes;
    sent(&built).await;
    let kept = &read(&built).unwrap().attachments[0].bytes;
    assert_eq!(kept.as_slice(), b"Subject: hi\r\nFrom: c@example.com\r\n\r\nhello\r\nbye\r\n");
}

#[tokio::test]
async fn a_multipart_with_lf_lines_goes_out_in_crlf() {
    let part = Part {
        name: Some("parts".into()),
        content_type: "multipart/mixed".into(),
        params: vec![("boundary".into(), "b".into())],
        content_id: None,
        bytes: b"Content-Type: multipart/mixed; boundary=b\n\n--b\nContent-Type: text/plain\n\nx\n--b--\n".to_vec(),
        section: None,
    };
    let built = build(&draft(part)).unwrap().bytes;
    sent(&built).await;
    let kept = read(&built).unwrap();
    assert_eq!(kept.attachments[0].content_type, "multipart/mixed");
}
