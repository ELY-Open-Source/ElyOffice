use std::time::Duration;

use crate::Stage;
use base64::{Engine, engine::general_purpose::STANDARD};
use mail_send::{
    Credentials, Error as SendError, SmtpClient,
    smtp::{AssertReply, message::Parameters},
};
use tokio::io::AsyncWriteExt;
use tokio_rustls::TlsConnector;

use crate::{
    Error, Result,
    net::{self, Login, Secret, Security, Server, Stream, within},
};

/// EHLO name: an address literal, so no host name leaks.
const HELLO: &str = "[127.0.0.1]";
const WAIT: Duration = Duration::from_secs(60);

/// A logged-in SMTP session over TLS.
pub struct Smtp {
    client: SmtpClient<Stream>,
    /// The server takes UTF-8 addresses (SMTPUTF8).
    utf8: bool,
    /// The server takes 8-bit bodies (8BITMIME).
    eight: bool,
    /// RFC 1870: the largest message it takes.
    limit: Option<u64>,
}

impl Smtp {
    pub async fn connect(server: &Server, login: &Login, tls: &TlsConnector) -> Result<Self> {
        let tcp = net::tcp(server).await?;
        let mut client = match server.security {
            Security::Tls => {
                let mut client = smtp(net::tls(tcp, server, tls).await?);
                greeting(&mut client).await?;
                client
            }
            Security::StartTls => {
                let mut plain = smtp(tcp);
                greeting(&mut plain).await?;
                let hello = within(Stage::Greeting, plain.ehlo(HELLO))
                    .await?
                    .map_err(failed(Stage::Greeting))?;
                if !hello.has_capability(smtp_proto::EXT_START_TLS) {
                    return Err(Error::Server("the server does not offer STARTTLS".into()));
                }
                within(Stage::StartingTls, plain.start_tls(tls, &server.host))
                    .await?
                    .map_err(failed(Stage::StartingTls))?
            }
        };
        let hello = within(Stage::Greeting, client.capabilities(HELLO, false))
            .await?
            .map_err(failed(Stage::Greeting))?;
        match &login.secret {
            Secret::Password(password) => {
                let credentials = Credentials::Plain {
                    username: login.user.clone(),
                    secret: password.clone(),
                };
                within(Stage::LoggingIn, client.authenticate(&credentials, &hello))
                    .await?
                    .map_err(refused)?;
            }
            Secret::Token(token) => {
                if hello.auth_mechanisms & smtp_proto::AUTH_XOAUTH2 == 0 {
                    return Err(Error::Rejected("the server does not offer XOAUTH2".into()));
                }
                within(Stage::LoggingIn, xoauth2(&mut client, &login.xoauth2(token))).await??;
            }
        }
        log::info!("mail: SMTP login on {}", server.host);
        let utf8 = hello.has_capability(smtp_proto::EXT_SMTP_UTF8);
        let eight = hello.has_capability(smtp_proto::EXT_8BIT_MIME);
        // SIZE 0 is no fixed limit.
        let limit = match hello.has_capability(smtp_proto::EXT_SIZE) && hello.size > 0 {
            true => Some(hello.size as u64),
            false => None,
        };
        Ok(Self {
            client,
            utf8,
            eight,
            limit,
        })
    }

    pub fn limit(&self) -> Option<u64> {
        self.limit
    }

    /// Hands one message to the server for these recipients.
    pub async fn send(&mut self, from: &str, to: &[String], message: &[u8]) -> Result<()> {
        if to.is_empty() {
            return Err(Error::Invalid("a message needs a recipient".into()));
        }
        for email in to.iter().map(String::as_str).chain([from]) {
            mailbox(email)?;
        }
        if let Some(limit) = self.limit
            && message.len() as u64 > limit
        {
            let size = message.len() as u64;
            return Err(Error::TooLarge { size, limit });
        }
        let data = data(message)?;
        let headers = crate::mime::utf8_headers(message)?;
        let mut params = Parameters::new();
        let body = message
            .windows(4)
            .position(|end| end == b"\r\n\r\n")
            .map_or(&[][..], |at| &message[at..]);
        if !body.is_ascii() {
            if !self.eight {
                return Err(Error::Invalid(
                    "the server does not take 8-bit message bodies".into(),
                ));
            }
            params.add(("BODY", "8BITMIME"));
        }
        if !from.is_ascii() || to.iter().any(|to| !to.is_ascii()) || headers {
            if !self.utf8 {
                return Err(Error::Invalid(
                    "the server does not take non-ASCII addresses or headers".into(),
                ));
            }
            params.add("SMTPUTF8");
        }
        let handed = within(Stage::Sending, async {
            self.client.mail_from(from, &params).await?;
            for to in to {
                self.client.rcpt_to(to, &Parameters::new()).await?;
            }
            self.client.cmd(b"DATA\r\n").await?.assert_code(354)?;
            self.client.stream.write_all(&data).await?;
            Ok::<_, SendError>(())
        })
        .await?;
        match handed {
            Err(SendError::UnexpectedReply(reply)) => return Err(refusal(reply)),
            other => other.map_err(failed(Stage::Sending))?,
        }
        // The final dot may be out already.
        match within(Stage::Sending, self.client.stream.flush()).await {
            Ok(Ok(())) => {}
            Ok(Err(err)) => return Err(Error::Unsure(err.to_string())),
            Err(err) => return Err(Error::Unsure(err.to_string())),
        }
        // The message is out; a lost answer leaves it unknown.
        let answer = match within(Stage::WaitingToAccept, self.client.read()).await {
            Ok(Ok(answer)) => answer,
            Ok(Err(err)) => return Err(Error::Unsure(failed(Stage::WaitingToAccept)(err).to_string())),
            Err(err) => return Err(Error::Unsure(err.to_string())),
        };
        if let Err(SendError::UnexpectedReply(reply)) = answer.assert_positive_completion() {
            return Err(refusal(reply));
        }
        log::info!("mail: SMTP accepted a message for {} recipients", to.len());
        Ok(())
    }

    pub async fn quit(self) -> Result<()> {
        within(Stage::LoggingOut, self.client.quit())
            .await?
            .map_err(failed(Stage::LoggingOut))
    }
}

/// The transaction stays open; only QUIT follows.
fn refusal(reply: smtp_proto::Response<String>) -> Error {
    Error::Server(format!("{} {}", reply.code(), reply.message()))
}

fn smtp<T: tokio::io::AsyncRead + tokio::io::AsyncWrite>(stream: T) -> SmtpClient<T> {
    SmtpClient {
        stream,
        timeout: WAIT,
    }
}

async fn greeting<T>(client: &mut SmtpClient<T>) -> Result<()>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let reply = within(Stage::WaitingForGreeting, client.read())
        .await?
        .map_err(failed(Stage::WaitingForGreeting))?;
    reply.assert_positive_completion().map_err(failed(Stage::WaitingForGreeting))
}

/// Gmail's exchange: an error challenge draws an empty line.
async fn xoauth2<T>(client: &mut SmtpClient<T>, initial: &str) -> Result<()>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let command = format!("AUTH XOAUTH2 {}\r\n", STANDARD.encode(initial));
    let mut reply = client.cmd(command).await.map_err(failed(Stage::LoggingIn))?;
    if reply.code() == 334 {
        reply = client.cmd("\r\n").await.map_err(failed(Stage::LoggingIn))?;
    }
    match reply.code() {
        235 => Ok(()),
        code => Err(Error::Rejected(format!("{code} {}", reply.message()))),
    }
}

/// RFC 5321 §4.5.2: a leading dot doubles.
fn data(message: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(message.len() + 8);
    let mut start = true;
    for (at, &byte) in message.iter().enumerate() {
        let bare = match byte {
            b'\r' => message.get(at + 1) != Some(&b'\n'),
            b'\n' => at == 0 || message[at - 1] != b'\r',
            _ => false,
        };
        if bare {
            return Err(Error::Invalid("the message has a bare CR or LF".into()));
        }
        if start && byte == b'.' {
            out.push(b'.');
        }
        out.push(byte);
        start = byte == b'\n';
    }
    if !out.is_empty() && !out.ends_with(b"\r\n") {
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b".\r\n");
    Ok(out)
}

/// RFC 5321 §4.1.2 Mailbox, with RFC 6531 UTF-8.
pub(crate) fn mailbox(email: &str) -> Result<()> {
    let readable = email.rsplit_once('@').is_some_and(|(local, domain)| {
        let local = match local
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
        {
            Some(inner) => quoted(inner),
            None => dotted(local),
        };
        local && domain_ok(domain)
    });
    if !readable {
        return Err(Error::Invalid(format!("not a mail address: {email:?}")));
    }
    Ok(())
}

fn quoted(inner: &str) -> bool {
    let mut chars = inner.chars();
    while let Some(char) = chars.next() {
        let fine = match char {
            '\\' => chars.next().is_some_and(|next| matches!(next, ' '..='~')),
            '"' => false,
            ' '..='~' => true,
            _ => !char.is_ascii(),
        };
        if !fine {
            return false;
        }
    }
    true
}

fn dotted(local: &str) -> bool {
    let atext = |char: char| {
        char.is_ascii_alphanumeric() || "!#$%&'*+-/=?^_`{|}~".contains(char) || !char.is_ascii()
    };
    local
        .split('.')
        .all(|atom| !atom.is_empty() && atom.chars().all(atext))
}

fn domain_ok(domain: &str) -> bool {
    if let Some(literal) = domain
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    {
        return !literal.is_empty()
            && literal
                .bytes()
                .all(|byte| matches!(byte, 33..=90 | 94..=126));
    }
    domain.split('.').all(|label| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .chars()
                .all(|char| char.is_ascii_alphanumeric() || char == '-' || !char.is_ascii())
    })
}

fn refused(err: SendError) -> Error {
    match err {
        SendError::AuthenticationFailed(reply) => {
            Error::Rejected(format!("{} {}", reply.code(), reply.message()))
        }
        SendError::UnsupportedAuthMechanism => {
            Error::Rejected("the server offers no login method for these credentials".into())
        }
        other => failed(Stage::LoggingIn)(other),
    }
}

/// Mail-send's own timer names the step it ran in.
fn failed(stage: Stage) -> impl Fn(SendError) -> Error {
    move |err| match err {
        SendError::Io(err) => Error::Connect(err),
        SendError::Tls(err) => Error::Tls(err.to_string()),
        SendError::Timeout => Error::Timeout(stage),
        SendError::UnexpectedReply(reply) => {
            Error::Server(format!("{} {}", reply.code(), reply.message()))
        }
        other => Error::Protocol(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mail_sends_own_timeout_keeps_the_step() {
        let quiet = |error| match error {
            Error::Timeout(stage) => stage,
            other => panic!("not a timeout: {other}"),
        };
        assert_eq!(quiet(failed(Stage::StartingTls)(SendError::Timeout)), Stage::StartingTls);
        assert_eq!(quiet(refused(SendError::Timeout)), Stage::LoggingIn);
    }
}
