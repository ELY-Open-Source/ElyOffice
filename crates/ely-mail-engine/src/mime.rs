use mail_builder::{
    MessageBuilder,
    headers::{address::Address as Out, content_type::ContentType, message_id::MessageId},
    mime::MimePart,
};
use mail_parser::{Address, HeaderValue, MessageParser};
use serde::{Deserialize, Serialize};

use crate::{Error, Result, smtp::mailbox};

mod walk;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mailbox {
    pub name: Option<String>,
    pub email: String,
}

/// An attachment or a `cid:` target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    pub name: Option<String>,
    pub content_type: String,
    /// Content-Type parameters, such as charset.
    pub params: Vec<(String, String)>,
    pub content_id: Option<String>,
    /// Transfer-decoded; a multipart keeps its headers.
    pub bytes: Vec<u8>,
    /// Its IMAP part number, for a part as read.
    pub section: Option<String>,
}

/// A message as read; bodies are the sender's own parts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mail {
    pub subject: Option<String>,
    pub from: Vec<Mailbox>,
    /// Where answers go instead of From.
    pub reply_to: Vec<Mailbox>,
    pub to: Vec<Mailbox>,
    pub cc: Vec<Mailbox>,
    /// Seconds since the Unix epoch.
    pub date: Option<i64>,
    pub message_id: Option<String>,
    pub in_reply_to: Vec<String>,
    pub references: Vec<String>,
    pub text: Option<String>,
    pub html: Option<String>,
    /// Bodies to read in order; one per alternative.
    pub shown: Vec<Shown>,
    pub inline: Vec<Part>,
    pub attachments: Vec<Part>,
    /// The header block as it arrived.
    pub headers: String,
}

/// One body part, as the reader reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shown {
    Text(String),
    Html(String),
}

/// A message to write; sent Bcc stays in the envelope.
#[derive(Clone, Debug)]
pub struct Draft {
    pub from: Mailbox,
    pub to: Vec<Mailbox>,
    pub cc: Vec<Mailbox>,
    /// Written as a header: for draft copies only.
    pub bcc: Vec<Mailbox>,
    pub subject: String,
    pub text: String,
    pub html: Option<String>,
    pub attachments: Vec<Part>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    /// Seconds since the Unix epoch.
    pub date: i64,
}

pub struct Built {
    /// Without angle brackets, as `Mail::message_id` reads it.
    pub message_id: String,
    pub bytes: Vec<u8>,
}

pub fn read(raw: &[u8]) -> Result<Mail> {
    let message = MessageParser::default()
        .parse(raw)
        .ok_or_else(|| Error::Mime("not a mail message".into()))?;
    let problems = message
        .parts
        .iter()
        .filter(|part| part.is_encoding_problem)
        .count();
    if problems > 0 {
        log::warn!("mail: {problems} MIME parts did not decode cleanly");
    }
    if let Err(err) = walk::depth(&message) {
        walk::dispose(message);
        return Err(err);
    }
    let header_end = message.root_part().offset_body as usize;
    let shape = walk::shape(&message);
    Ok(Mail {
        subject: message.subject().map(str::to_owned),
        from: mailboxes(message.from().into_iter()),
        reply_to: mailboxes(message.reply_to().into_iter()),
        to: mailboxes(message.all_to()),
        cc: mailboxes(message.all_cc()),
        date: message.date().map(|date| date.to_timestamp()),
        message_id: message.message_id().map(str::to_owned),
        in_reply_to: ids(message.in_reply_to()),
        references: ids(message.references()),
        text: joined(shape.text),
        html: joined(shape.html),
        shown: shape.shown,
        inline: shape.inline,
        attachments: shape.attachments,
        headers: String::from_utf8_lossy(&raw[..header_end.min(raw.len())]).into_owned(),
    })
}

/// Whether any header block needs SMTPUTF8.
pub(crate) fn utf8_headers(raw: &[u8]) -> Result<bool> {
    let message = MessageParser::default()
        .parse(raw)
        .ok_or_else(|| Error::Mime("not a mail message".into()))?;
    let checked = walk::depth(&message).map(|()| walk::utf8_headers(&message));
    walk::dispose(message);
    checked
}

pub fn build(draft: &Draft) -> Result<Built> {
    let people = [&draft.from].into_iter().chain(&draft.to).chain(&draft.cc);
    let people = people.chain(&draft.bcc);
    for person in people {
        mailbox(&person.email)?;
        line("a name", person.name.as_deref().unwrap_or_default())?;
    }
    line("the subject", &draft.subject)?;
    for id in draft.in_reply_to.iter().chain(&draft.references) {
        line("a message ID", id)?;
    }
    let message_id = message_id(&draft.from.email)?;
    let mut builder = MessageBuilder::new()
        .message_id(message_id.as_str())
        .date(draft.date)
        .from(address(&draft.from))
        .subject(draft.subject.as_str())
        .body(body(draft)?);
    if !draft.to.is_empty() {
        builder = builder.to(Out::new_list(draft.to.iter().map(address).collect()));
    }
    if !draft.cc.is_empty() {
        builder = builder.cc(Out::new_list(draft.cc.iter().map(address).collect()));
    }
    if !draft.bcc.is_empty() {
        builder = builder.bcc(Out::new_list(draft.bcc.iter().map(address).collect()));
    }
    if let Some(id) = &draft.in_reply_to {
        builder = builder.in_reply_to(id.as_str());
    }
    if !draft.references.is_empty() {
        let ids = draft.references.iter().map(String::as_str);
        builder = builder.references(MessageId::new_list(ids));
    }
    let bytes = builder
        .write_to_vec()
        .map_err(|err| Error::Mime(err.to_string()))?;
    Ok(Built { message_id, bytes })
}

/// The sent form: the top header block loses Bcc.
pub fn without_bcc(raw: &[u8]) -> Result<Vec<u8>> {
    let end = raw
        .windows(4)
        .position(|end| end == b"\r\n\r\n")
        .ok_or_else(|| Error::Mime("the message has no header end".into()))?;
    let (head, body) = raw.split_at(end + 2);
    let mut out = Vec::with_capacity(raw.len());
    let mut dropping = false;
    for line in head.split_inclusive(|byte| *byte == b'\n') {
        // RFC 5322 §2.2.3: a fold starts with a space.
        if !line.starts_with(b" ") && !line.starts_with(b"\t") {
            let mut name = line.split(|byte| *byte == b':');
            dropping = name
                .next()
                .is_some_and(|it| it.eq_ignore_ascii_case(b"Bcc"));
        }
        if !dropping {
            out.extend_from_slice(line);
        }
    }
    out.extend_from_slice(body);
    Ok(out)
}

/// Mixed of the alternative bodies and the attachments.
fn body(draft: &Draft) -> Result<MimePart<'_>> {
    let text = MimePart::new("text/plain", draft.text.as_str());
    let body = match &draft.html {
        Some(html) => {
            let html = MimePart::new("text/html", html.as_str());
            MimePart::new("multipart/alternative", vec![text, html])
        }
        None => text,
    };
    if draft.attachments.is_empty() {
        return Ok(body);
    }
    let mut parts = vec![body];
    for part in &draft.attachments {
        parts.push(attachment(part)?);
    }
    Ok(MimePart::new("multipart/mixed", parts))
}

fn attachment(part: &Part) -> Result<MimePart<'_>> {
    if let Some(name) = &part.name {
        line("a file name", name)?;
    }
    line("a content type", &part.content_type)?;
    let mut kind = ContentType::new(part.content_type.as_str());
    for (key, value) in &part.params {
        line("a content type parameter", key)?;
        line("a content type parameter", value)?;
        kind = kind.attribute(key.as_str(), value.as_str());
    }
    let composite = ["multipart/", "message/"]
        .iter()
        .any(|prefix| part.content_type.starts_with(prefix));
    let mut attachment = if composite {
        // RFC 2045 §6.4: composites stay unencoded.
        let body = composite_body(part)?;
        let encoding = if body.is_ascii() { "7bit" } else { "8bit" };
        MimePart::new(kind, body).transfer_encoding(encoding)
    } else {
        MimePart::new(kind, part.bytes.as_slice())
    };
    attachment = match &part.name {
        Some(name) => attachment.attachment(name.as_str()),
        // RFC 2183 §2.3: a name is optional.
        None => attachment.header("Content-Disposition", ContentType::new("attachment")),
    };
    if let Some(id) = &part.content_id {
        line("a content ID", id)?;
        attachment = attachment.cid(id.as_str());
    }
    Ok(attachment)
}

/// In CRLF; a multipart as read carries its headers.
fn composite_body(part: &Part) -> Result<Vec<u8>> {
    let bytes = crlf(&part.bytes);
    if !part.content_type.starts_with("multipart/") {
        return Ok(bytes);
    }
    if !part.params.iter().any(|(key, _)| key == "boundary") {
        return Err(Error::Invalid(
            "a multipart attachment has no boundary".into(),
        ));
    }
    if let Some(body) = bytes.strip_prefix(b"\r\n") {
        return Ok(body.to_vec());
    }
    bytes
        .windows(4)
        .position(|end| end == b"\r\n\r\n")
        .map(|at| bytes[at + 4..].to_vec())
        .ok_or_else(|| Error::Invalid("a multipart attachment has no header end".into()))
}

/// RFC 2049 §4: lone CR and LF become CRLF.
fn crlf(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'\r' if bytes.get(at + 1) == Some(&b'\n') => {
                out.extend_from_slice(b"\r\n");
                at += 1;
            }
            b'\r' | b'\n' => out.extend_from_slice(b"\r\n"),
            byte => out.push(byte),
        }
        at += 1;
    }
    out
}

/// A random left side at the sender's domain.
fn message_id(from: &str) -> Result<String> {
    let (_, domain) = from
        .rsplit_once('@')
        .ok_or_else(|| Error::Invalid(format!("not a mail address: {from:?}")))?;
    let mut random = [0u8; 16];
    getrandom::fill(&mut random).map_err(|err| Error::Mime(err.to_string()))?;
    let left: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!("{left}@{domain}"))
}

/// A line break would start a new header.
fn line(what: &str, text: &str) -> Result<()> {
    if text.contains(['\r', '\n']) {
        return Err(Error::Invalid(format!("{what} has a line break")));
    }
    Ok(())
}

fn address(mailbox: &Mailbox) -> Out<'_> {
    Out::new_address(mailbox.name.as_deref(), mailbox.email.as_str())
}

fn mailboxes<'a>(lists: impl Iterator<Item = &'a Address<'a>>) -> Vec<Mailbox> {
    lists
        .flat_map(Address::iter)
        .filter_map(|addr| {
            let Some(email) = addr.address.as_deref() else {
                log::warn!("mail: an address without an email was skipped");
                return None;
            };
            Some(Mailbox {
                name: addr.name.as_deref().map(str::to_owned),
                email: email.to_owned(),
            })
        })
        .collect()
}

fn ids(value: &HeaderValue) -> Vec<String> {
    match value {
        HeaderValue::Text(id) => vec![id.to_string()],
        HeaderValue::TextList(ids) => ids.iter().map(ToString::to_string).collect(),
        _ => Vec::new(),
    }
}

fn joined(parts: Vec<String>) -> Option<String> {
    (!parts.is_empty()).then(|| parts.join("\n"))
}
