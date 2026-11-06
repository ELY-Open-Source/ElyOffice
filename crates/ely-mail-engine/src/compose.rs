//! Answers, forwards and the bodies they send (C12.12, MAIL-05).

use crate::{Mail, Mailbox, Part, page};
use serde::{Deserialize, Serialize};

/// Where an answer or a forward starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Start {
    pub to: Vec<Mailbox>,
    pub cc: Vec<Mailbox>,
    pub subject: String,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
}

/// To its writer; with `all`, to everyone but `me`.
pub fn answer(mail: &Mail, me: &str, all: bool) -> Start {
    let mine = |person: &Mailbox| person.email.eq_ignore_ascii_case(me);
    let sent = mail.from.iter().any(mine);
    // One's own message answers its recipients.
    let first = match (sent, mail.reply_to.is_empty()) {
        (true, _) => &mail.to,
        (false, true) => &mail.from,
        (false, false) => &mail.reply_to,
    };
    let mut seen: Vec<String> = Vec::new();
    let mut fresh = |person: &&Mailbox| {
        let email = person.email.to_ascii_lowercase();
        let new = !mine(person) && !seen.contains(&email);
        seen.push(email);
        new
    };
    let mut to: Vec<Mailbox> = first.iter().filter(&mut fresh).cloned().collect();
    let mut cc = Vec::new();
    if all {
        if !sent {
            to.extend(mail.to.iter().filter(&mut fresh).cloned());
        }
        cc = mail.cc.iter().filter(&mut fresh).cloned().collect();
    }
    if to.is_empty() && cc.is_empty() {
        // A note to oneself answers oneself.
        to = first.clone();
    }
    let mut references = match (mail.references.is_empty(), mail.in_reply_to.as_slice()) {
        (false, _) => mail.references.clone(),
        (true, [parent]) => vec![parent.clone()],
        (true, _) => Vec::new(),
    };
    references.extend(mail.message_id.iter().cloned());
    Start {
        to,
        cc,
        subject: prefixed(mail.subject.as_deref(), "Re: ", &["re:"]),
        in_reply_to: mail.message_id.clone(),
        references,
    }
}

/// To nobody yet, under the same subject.
pub fn forward(mail: &Mail) -> Start {
    Start {
        to: Vec::new(),
        cc: Vec::new(),
        subject: prefixed(mail.subject.as_deref(), "Fwd: ", &["fwd:", "fw:"]),
        in_reply_to: None,
        references: Vec::new(),
    }
}

/// Whether `email` is a mailbox SMTP takes.
pub fn is_address(email: &str) -> bool {
    crate::smtp::mailbox(email).is_ok()
}

/// A file to send, typed by its name.
pub fn attachment(name: &str, bytes: Vec<u8>) -> Part {
    // RFC 2046 §4.5.1: unknown bytes are octet-stream.
    let kind = mime_guess::from_path(name).first_or_octet_stream();
    Part {
        name: Some(name.to_owned()),
        content_type: kind.essence_str().to_owned(),
        params: Vec::new(),
        content_id: None,
        bytes,
        section: None,
    }
}

/// Its words as the pane shows them, hidden ones out.
pub fn original(mail: &Mail) -> String {
    page::read(&mail.shown, &mail.inline).text()
}

/// Adds `prefix` unless one of `known` starts it.
fn prefixed(subject: Option<&str>, prefix: &str, known: &[&str]) -> String {
    let subject = subject.unwrap_or_default().trim();
    let lower = subject.to_lowercase();
    match known.iter().any(|mark| lower.starts_with(mark)) {
        true => subject.to_owned(),
        false => format!("{prefix}{subject}").trim_end().to_owned(),
    }
}

/// The earlier message under an answer or a forward.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Earlier {
    /// An answer's: a lead line, then the words quoted.
    Quoted { lead: String, text: String },
    /// A forward's: its header lines, then the words.
    Forwarded { lead: Vec<String>, text: String },
}

/// What the writer wrote, and what goes under it.
pub struct Writing<'a> {
    pub words: &'a str,
    /// The words as HTML; plain text only without.
    pub html: Option<&'a str>,
    /// Empty for none.
    pub signature: &'a str,
    pub earlier: Option<&'a Earlier>,
}

/// A message's plain text, and its HTML for Markdown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bodies {
    pub text: String,
    pub html: Option<String>,
}

pub fn bodies(writing: &Writing) -> Bodies {
    let words = writing.words.trim_end();
    let signature = writing.signature.trim();
    let mut text = Vec::new();
    if !words.is_empty() {
        text.push(words.to_owned());
    }
    if !signature.is_empty() {
        // RFC 3676 §4.3: dash, dash, space.
        text.push(format!("-- \n{signature}"));
    }
    match writing.earlier {
        Some(Earlier::Quoted {
            lead,
            text: earlier,
        }) => {
            text.push(format!("{lead}\n{}", quoted(earlier)));
        }
        Some(Earlier::Forwarded {
            lead,
            text: earlier,
        }) => {
            text.push(format!("{}\n\n{earlier}", lead.join("\n")));
        }
        None => {}
    }
    let text = text.join("\n\n");
    let Some(html) = writing.html else {
        return Bodies { text, html: None };
    };
    let mut html = html.to_owned();
    if !signature.is_empty() {
        html.push_str(&format!("<p>-- <br>\n{}</p>\n", lines(signature)));
    }
    match writing.earlier {
        Some(Earlier::Quoted { lead, text: earlier }) => html.push_str(&format!(
            "<p>{}</p>\n<blockquote style=\"margin:0 0 0 .8ex;border-left:1px solid #ccc;padding-left:1ex\">\n{}</blockquote>\n",
            escape(lead),
            lines(earlier)
        )),
        Some(Earlier::Forwarded { lead, text: earlier }) => html.push_str(&format!(
            "<p>{}</p>\n<p>{}</p>\n",
            lines(&lead.join("\n")),
            lines(earlier)
        )),
        None => {}
    }
    Bodies {
        text,
        html: Some(html),
    }
}

/// Each line behind `> `; a blank one behind `>`.
fn quoted(text: &str) -> String {
    text.lines()
        .map(|line| match line.is_empty() {
            true => ">".to_owned(),
            false => format!("> {line}"),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Escaped lines joined by line breaks.
fn lines(text: &str) -> String {
    text.lines().map(escape).collect::<Vec<_>>().join("<br>\n")
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}
