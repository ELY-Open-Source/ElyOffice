use std::io;

use crate::Stage;
use base64::{Engine, engine::general_purpose::STANDARD};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio_rustls::TlsConnector;

use crate::{
    Error, Result,
    net::{self, Login, Secret, Security, Server, Stream, within},
};

/// A logged-in POP3 session (RFC 1939) over TLS.
pub struct Pop {
    io: BufReader<Stream>,
}

impl Pop {
    pub async fn connect(server: &Server, login: &Login, tls: &TlsConnector) -> Result<Self> {
        let tcp = net::tcp(server).await?;
        let stream = match server.security {
            Security::Tls => net::tls(tcp, server, tls).await?,
            Security::StartTls => {
                let mut plain = BufReader::new(tcp);
                within(Stage::WaitingForGreeting, status(&mut plain)).await??;
                within(Stage::StartingTls, ask(&mut plain, "STLS")).await??;
                // Plain bytes here would pass as TLS data.
                if !plain.buffer().is_empty() {
                    return Err(Error::Protocol("the server sent data after STLS".into()));
                }
                net::tls(plain.into_inner(), server, tls).await?
            }
        };
        let mut io = BufReader::new(stream);
        if server.security == Security::Tls {
            within(Stage::WaitingForGreeting, status(&mut io)).await??;
        }
        within(Stage::LoggingIn, authenticate(&mut io, login)).await??;
        log::info!("mail: POP3 login on {}", server.host);
        Ok(Self { io })
    }

    /// Message numbers with their unique ids.
    pub async fn uids(&mut self) -> Result<Vec<(usize, String)>> {
        let list = within(Stage::ListingMessages, async {
            ask(&mut self.io, "UIDL").await?;
            lines(&mut self.io).await
        })
        .await??;
        list.split(|&byte| byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| {
                let line = std::str::from_utf8(line.strip_suffix(b"\r").unwrap_or(line))
                    .map_err(|_| Error::Protocol("UIDL sent a line that is not text".into()))?;
                let unreadable = || Error::Protocol(format!("UIDL sent {line:?}"));
                let (number, id) = line.split_once(' ').ok_or_else(unreadable)?;
                let number = number.parse().map_err(|_| unreadable())?;
                Ok((number, id.trim_end().to_owned()))
            })
            .collect()
    }

    pub async fn fetch(&mut self, number: usize) -> Result<Vec<u8>> {
        within(Stage::FetchingMessage, async {
            ask(&mut self.io, &format!("RETR {number}")).await?;
            lines(&mut self.io).await
        })
        .await?
    }

    /// Marks it deleted; QUIT carries that out.
    pub async fn delete(&mut self, number: usize) -> Result<()> {
        within(
            Stage::DeletingMessage,
            ask(&mut self.io, &format!("DELE {number}")),
        )
        .await?
    }

    pub async fn quit(mut self) -> Result<()> {
        within(Stage::LoggingOut, ask(&mut self.io, "QUIT")).await??;
        Ok(())
    }
}

async fn authenticate<S>(io: &mut BufReader<S>, login: &Login) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    match &login.secret {
        Secret::Password(password) => {
            one_line(&login.user)?;
            one_line(password)?;
            ask(io, &format!("USER {}", login.user))
                .await
                .map_err(refused)?;
            ask(io, &format!("PASS {password}"))
                .await
                .map_err(refused)?;
        }
        Secret::Token(token) => {
            send(io, "AUTH XOAUTH2").await?;
            if status(io).await.map_err(refused)?.is_none() {
                return Err(Error::Protocol(
                    "XOAUTH2 was accepted before the token".into(),
                ));
            }
            send(io, &STANDARD.encode(login.xoauth2(token))).await?;
            if status(io).await.map_err(refused)?.is_some() {
                // An error challenge; the empty answer draws -ERR.
                send(io, "").await?;
                status(io).await.map_err(refused)?;
                return Err(Error::Protocol(
                    "the token was challenged, then accepted".into(),
                ));
            }
        }
    }
    Ok(())
}

/// Text after `+OK`, or `Some` after a `+` challenge.
async fn status<S>(io: &mut BufReader<S>) -> Result<Option<String>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let line = line(io).await?;
    let words = |rest: &[u8]| String::from_utf8_lossy(rest).trim().to_owned();
    if line.starts_with(b"+OK") {
        return Ok(None);
    }
    if let Some(rest) = line.strip_prefix(b"-ERR") {
        return Err(Error::Server(words(rest)));
    }
    if let Some(rest) = line.strip_prefix(b"+") {
        return Ok(Some(words(rest)));
    }
    Err(Error::Protocol(
        "the server's line was neither +OK nor -ERR".into(),
    ))
}

/// One command that must end in `+OK`.
async fn ask<S>(io: &mut BufReader<S>, command: &str) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    send(io, command).await?;
    match status(io).await? {
        None => Ok(()),
        Some(_) => Err(Error::Protocol(
            "the server sent a challenge to a command".into(),
        )),
    }
}

async fn send<S>(io: &mut BufReader<S>, command: &str) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let stream = io.get_mut();
    stream
        .write_all(command.as_bytes())
        .await
        .map_err(Error::Connect)?;
    stream.write_all(b"\r\n").await.map_err(Error::Connect)?;
    stream.flush().await.map_err(Error::Connect)
}

async fn line<S>(io: &mut BufReader<S>) -> Result<Vec<u8>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut line = Vec::new();
    io.read_until(b'\n', &mut line)
        .await
        .map_err(Error::Connect)?;
    if line.last() != Some(&b'\n') {
        return Err(Error::Connect(io::ErrorKind::UnexpectedEof.into()));
    }
    Ok(line)
}

/// A multi-line answer, dot-unstuffed, line ends kept.
async fn lines<S>(io: &mut BufReader<S>) -> Result<Vec<u8>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut body = Vec::new();
    loop {
        let line = line(io).await?;
        if line == b".\r\n" || line == b".\n" {
            return Ok(body);
        }
        body.extend_from_slice(line.strip_prefix(b".").unwrap_or(&line));
    }
}

/// A line break would start a second command.
fn one_line(text: &str) -> Result<()> {
    if text.contains(['\r', '\n']) {
        return Err(Error::Invalid(
            "a user name or password holds a line break".into(),
        ));
    }
    Ok(())
}

fn refused(err: Error) -> Error {
    match err {
        Error::Server(text) => Error::Rejected(text),
        other => other,
    }
}
