use std::collections::HashMap;

use imap_proto::{AttributeValue, Capability, MailboxDatum, NameAttribute, Response, ResponseCode};
use tokio_rustls::TlsConnector;

use crate::{
    Error, Result,
    net::{self, Login, Secret, Security, Server, Stream, within},
};

mod conn;
mod drafts;
mod organize;
mod parts;
pub(crate) mod utf7;

use conn::{Arg, Conn};
pub use organize::Can;
pub use parts::Node;

/// A logged-in IMAP session over TLS.
pub struct Imap {
    conn: Conn<Stream>,
}

/// What a folder is for (RFC 6154; INBOX by name).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Inbox,
    Sent,
    Drafts,
    Trash,
    Junk,
    Archive,
    All,
    Flagged,
}

impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Role::Inbox => "inbox",
            Role::Sent => "sent",
            Role::Drafts => "drafts",
            Role::Trash => "trash",
            Role::Junk => "junk",
            Role::Archive => "archive",
            Role::All => "all",
            Role::Flagged => "flagged",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [
            Role::Inbox,
            Role::Sent,
            Role::Drafts,
            Role::Trash,
            Role::Junk,
            Role::Archive,
            Role::All,
            Role::Flagged,
        ]
        .into_iter()
        .find(|role| role.name() == name)
    }
}

/// One listed folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folder {
    pub name: String,
    pub role: Option<Role>,
    /// False for `\Noselect` and `\NonExistent`.
    pub selectable: bool,
}

/// A folder opened read-only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opened {
    pub uidvalidity: u32,
    pub exists: u32,
}

/// One fetched message: header, text or whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Got {
    pub uid: u32,
    pub flags: Vec<String>,
    pub bytes: Vec<u8>,
}

impl Imap {
    pub async fn connect(server: &Server, login: &Login, tls: &TlsConnector) -> Result<Self> {
        let tcp = net::tcp(server).await?;
        let stream = match server.security {
            Security::Tls => net::tls(tcp, server, tls).await?,
            Security::StartTls => {
                let mut plain = Conn::new(tcp);
                within("waiting for the greeting", plain.greeting()).await??;
                within("starting TLS", plain.run("STARTTLS", &[], |_, _| {}))
                    .await??
                    .map_err(Error::Server)?;
                net::tls(plain.wire.into_inner()?, server, tls).await?
            }
        };
        let mut conn = Conn::new(stream);
        if server.security == Security::Tls {
            within("waiting for the greeting", conn.greeting()).await??;
        }
        let outcome = match &login.secret {
            Secret::Password(password) => {
                let args = [Arg::Text(&login.user), Arg::Text(password)];
                within("logging in", conn.run("LOGIN", &args, |_, _| {})).await??
            }
            Secret::Token(token) => {
                within("logging in", conn.xoauth2(&login.xoauth2(token))).await??
            }
        };
        outcome.map_err(Error::Rejected)?;
        log::info!("mail: IMAP login on {}", server.host);
        Ok(Self { conn })
    }

    /// Every folder the server lists, with its role.
    pub async fn folders(&mut self) -> Result<Vec<Folder>> {
        let can = self.capabilities().await?;
        let has = |name: &str| can.iter().any(|it| it.eq_ignore_ascii_case(name));
        // RFC 6154 §2: roles may come only on request.
        let extended = has("LIST-EXTENDED") && has("SPECIAL-USE");
        let mut folders = Vec::new();
        let all = [
            Arg::Text(""),
            Arg::Atom("*"),
            Arg::Atom("RETURN (SPECIAL-USE)"),
        ];
        let args = if extended { &all[..] } else { &all[..2] };
        within(
            "listing mailboxes",
            self.conn.run("LIST", args, |response, raw| {
                if let Response::MailboxData(MailboxDatum::List(list)) = response {
                    let name = mailbox_name(&list.name, raw);
                    folders.push(folder(name, &list.name_attributes));
                }
            }),
        )
        .await??
        .map_err(Error::Server)?;
        Ok(folders)
    }

    async fn capabilities(&mut self) -> Result<Vec<String>> {
        let mut can = Vec::new();
        within(
            "reading capabilities",
            self.conn.run("CAPABILITY", &[], |response, _| {
                if let Response::Capabilities(list) = response {
                    can.extend(list.iter().filter_map(|it| match it {
                        Capability::Atom(atom) => Some(atom.to_string()),
                        _ => None,
                    }));
                }
            }),
        )
        .await??
        .map_err(Error::Server)?;
        Ok(can)
    }

    pub async fn examine(&mut self, mailbox: &str) -> Result<Opened> {
        self.open("EXAMINE", "opening the mailbox", mailbox).await
    }

    /// EXAMINE reads; SELECT may change.
    async fn open(&mut self, verb: &str, step: &'static str, mailbox: &str) -> Result<Opened> {
        let (mut uidvalidity, mut exists) = (None, None);
        within(
            step,
            self.conn
                .run(verb, &[Arg::Text(mailbox)], |response, _| match response {
                    Response::Data { outcome, .. } => {
                        if let Some(ResponseCode::UidValidity(value)) = &outcome.code {
                            uidvalidity = Some(*value);
                        }
                    }
                    Response::MailboxData(MailboxDatum::Exists(count)) => exists = Some(*count),
                    _ => {}
                }),
        )
        .await??
        .map_err(Error::Server)?;
        let uidvalidity = uidvalidity
            .ok_or_else(|| Error::Protocol("the mailbox opened without a UIDVALIDITY".into()))?;
        // RFC 3501 §6.3.1: EXISTS is required.
        let exists =
            exists.ok_or_else(|| Error::Protocol("the mailbox opened without EXISTS".into()))?;
        Ok(Opened {
            uidvalidity,
            exists,
        })
    }

    /// Every UID in the open mailbox with its flags.
    pub async fn flags(&mut self) -> Result<Vec<(u32, Vec<String>)>> {
        let got = self
            .uid_fetch("1:*", "(UID FLAGS)", "reading flags")
            .await?;
        Ok(got.into_iter().map(|(uid, it)| (uid, it.flags)).collect())
    }

    /// Headers of `uids`; a UID gone meanwhile is absent.
    pub async fn headers(&mut self, uids: &[u32]) -> Result<Vec<Got>> {
        let what = "(UID FLAGS BODY.PEEK[HEADER])";
        let got = self.uid_fetch(&set(uids), what, "fetching headers").await?;
        asked(got, uids, "HEADER")
    }

    /// Whole messages of `uids`; a UID gone meanwhile is absent.
    pub async fn bodies(&mut self, uids: &[u32]) -> Result<Vec<Got>> {
        let what = "(UID FLAGS BODY.PEEK[])";
        let got = self
            .uid_fetch(&set(uids), what, "fetching messages")
            .await?;
        asked(got, uids, "")
    }

    /// Each UID's MIME tree; a UID gone meanwhile is absent.
    pub async fn structures(&mut self, uids: &[u32]) -> Result<Vec<(u32, Node)>> {
        let what = "(UID BODYSTRUCTURE)";
        let mut got = self
            .uid_fetch(&set(uids), what, "reading structures")
            .await?;
        let mut out = Vec::new();
        for &uid in uids {
            let Some(it) = got.remove(&uid) else {
                continue;
            };
            let node = it
                .structure
                .ok_or_else(|| Error::Protocol(format!("UID {uid} came without a structure")))?;
            out.push((uid, node?));
        }
        Ok(out)
    }

    /// The message without its left-out parts; `None` once gone.
    pub async fn text(&mut self, uid: u32, node: &Node) -> Result<Option<Got>> {
        let peeks: Vec<String> = node
            .sections()
            .iter()
            .map(|section| format!("BODY.PEEK[{section}]"))
            .collect();
        let what = format!("(UID FLAGS {})", peeks.join(" "));
        let mut got = self
            .uid_fetch(&uid.to_string(), &what, "fetching message text")
            .await?;
        let Some(it) = got.remove(&uid) else {
            return Ok(None);
        };
        Ok(Some(Got {
            uid,
            bytes: node.build(&it.sections)?,
            flags: it.flags,
        }))
    }

    /// UIDs of messages that arrived on or after `day` (`19-Jul-2026`).
    pub async fn since(&mut self, day: &str) -> Result<Vec<u32>> {
        let args = [Arg::Atom("SINCE"), Arg::Atom(day)];
        self.search(&args, "searching by date").await
    }

    /// UID SEARCH, sorted.
    async fn search(&mut self, args: &[Arg<'_>], step: &'static str) -> Result<Vec<u32>> {
        let mut uids = Vec::new();
        within(
            step,
            self.conn.run("UID SEARCH", args, |response, _| {
                if let Response::MailboxData(MailboxDatum::Search(found)) = response {
                    uids.extend_from_slice(found);
                }
            }),
        )
        .await??
        .map_err(Error::Server)?;
        uids.sort_unstable();
        Ok(uids)
    }

    /// The raw message; `None` once the UID is gone.
    pub async fn fetch(&mut self, mailbox: &str, uid: u32) -> Result<Option<Vec<u8>>> {
        self.examine(mailbox).await?;
        Ok(self.bodies(&[uid]).await?.pop().map(|got| got.bytes))
    }

    pub async fn logout(mut self) -> Result<()> {
        within("logging out", self.conn.run("LOGOUT", &[], |_, _| {}))
            .await??
            .map_err(Error::Server)
    }

    /// Every FETCH for a UID, merged; flags may come apart.
    async fn uid_fetch(
        &mut self,
        uids: &str,
        what: &str,
        step: &'static str,
    ) -> Result<HashMap<u32, Fetch>> {
        let mut got: HashMap<u32, Fetch> = HashMap::new();
        let args = [Arg::Atom(uids), Arg::Atom(what)];
        within(
            step,
            self.conn.run("UID FETCH", &args, |response, _| {
                if let Response::Fetch(_, values) = response {
                    merge(&mut got, values);
                }
            }),
        )
        .await??
        .map_err(Error::Server)?;
        Ok(got)
    }
}

#[derive(Default)]
struct Fetch {
    flags: Vec<String>,
    sections: HashMap<String, Vec<u8>>,
    structure: Option<Result<Node>>,
}

/// Unsolicited FETCH responses without a UID are skipped.
fn merge(got: &mut HashMap<u32, Fetch>, values: &[AttributeValue]) {
    let Some(uid) = values.iter().find_map(|value| match value {
        AttributeValue::Uid(uid) => Some(*uid),
        _ => None,
    }) else {
        return;
    };
    let it = got.entry(uid).or_default();
    for value in values {
        match value {
            AttributeValue::Flags(flags) => {
                // \Recent belongs to one session.
                it.flags = flags
                    .iter()
                    .filter(|flag| !flag.eq_ignore_ascii_case(r"\Recent"))
                    .map(|flag| flag.to_string())
                    .collect();
                it.flags.sort();
            }
            AttributeValue::BodySection {
                section,
                data: Some(data),
                ..
            } => {
                it.sections
                    .insert(parts::section_key(section), data.to_vec());
            }
            AttributeValue::BodyStructure(structure) => {
                it.structure = Some(Node::read(structure));
            }
            _ => {}
        }
    }
}

/// The asked UIDs in order; each needs `section`.
fn asked(mut got: HashMap<u32, Fetch>, uids: &[u32], section: &str) -> Result<Vec<Got>> {
    let mut out = Vec::new();
    for &uid in uids {
        let Some(mut it) = got.remove(&uid) else {
            continue;
        };
        let bytes = it
            .sections
            .remove(section)
            .ok_or_else(|| Error::Protocol(format!("UID {uid} came without {section:?}")))?;
        out.push(Got {
            uid,
            flags: it.flags,
            bytes,
        });
    }
    Ok(out)
}

/// `1:3,7` from sorted UIDs.
fn set(uids: &[u32]) -> String {
    let mut out = String::new();
    let mut at = 0;
    while at < uids.len() {
        let start = uids[at];
        let mut end = start;
        while at + 1 < uids.len() && uids[at + 1] == end + 1 {
            at += 1;
            end = uids[at];
        }
        if !out.is_empty() {
            out.push(',');
        }
        match start == end {
            true => out.push_str(&start.to_string()),
            false => out.push_str(&format!("{start}:{end}")),
        }
        at += 1;
    }
    out
}

fn folder(name: String, attributes: &[NameAttribute]) -> Folder {
    let mut role = name.eq_ignore_ascii_case("INBOX").then_some(Role::Inbox);
    let mut selectable = true;
    for attribute in attributes {
        match attribute {
            NameAttribute::NoSelect => selectable = false,
            NameAttribute::Extension(other) if other.eq_ignore_ascii_case("\\NonExistent") => {
                selectable = false
            }
            NameAttribute::Sent => role = Some(Role::Sent),
            NameAttribute::Drafts => role = Some(Role::Drafts),
            NameAttribute::Trash => role = Some(Role::Trash),
            NameAttribute::Junk => role = Some(Role::Junk),
            NameAttribute::Archive => role = Some(Role::Archive),
            NameAttribute::All => role = Some(Role::All),
            NameAttribute::Flagged => role = Some(Role::Flagged),
            _ => {}
        }
    }
    Folder {
        name,
        role,
        selectable,
    }
}

/// imap-proto keeps quoted escapes; a literal is as sent.
fn mailbox_name(name: &str, raw: &[u8]) -> String {
    let line = raw.strip_suffix(b"\r\n").unwrap_or(raw);
    let name_end = |end: &[u8], mark: &[u8]| {
        end.strip_suffix(name.as_bytes())
            .is_some_and(|rest| rest.ends_with(mark))
    };
    let literal = name_end(line, b"}\r\n");
    let quoted = line
        .strip_suffix(b"\"")
        .is_some_and(|end| name_end(end, b"\""));
    if literal || !quoted {
        return name.to_owned();
    }
    let mut plain = String::with_capacity(name.len());
    let mut chars = name.chars();
    while let Some(char) = chars.next() {
        match char {
            '\\' => plain.extend(chars.next()),
            _ => plain.push(char),
        }
    }
    plain
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_uid_set_joins_runs() {
        assert_eq!(super::set(&[1, 2, 3, 7, 9, 10]), "1:3,7,9:10");
        assert_eq!(super::set(&[5]), "5");
    }
}
