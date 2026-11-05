//! A scripted mail server over real TLS, for protocol tests.

pub mod oauth;
pub mod smtp;

use std::sync::Arc;

use ely_mail_engine::{Account, Auth, Login, Protocol, Secret, Security, Server, TlsConnector};
use rcgen::{CertificateParams, KeyPair};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
    net::TcpListener,
    task::JoinHandle,
};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        ClientConfig, RootCertStore, ServerConfig,
        crypto::ring,
        pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
    },
};

pub enum Step {
    /// CRLF for `\n`; `{tag}` is the client's last tag.
    Send(String),
    /// One line; a leading `{tag} ` takes the client's tag.
    Expect(String),
    /// SMTP DATA lines before the lone dot.
    Data(String),
    /// Any DATA, for messages built with a date.
    AnyData,
    /// One line that starts so; `{tag} ` as in `Expect`.
    Prefix(String),
    StartTls,
    /// Ends the script, holding the connection open unanswered.
    Hang,
    /// Runs at this point, as if the user acted now.
    Run(Box<dyn FnOnce() + Send>),
}

pub fn send(text: &str) -> Step {
    Step::Send(text.to_owned())
}

pub fn expect(text: &str) -> Step {
    Step::Expect(text.to_owned())
}

trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}

pub struct Certs {
    acceptor: TlsAcceptor,
    /// Trusts only this test certificate.
    pub trusted: TlsConnector,
}

pub fn certs() -> Certs {
    let key = KeyPair::generate().unwrap();
    let cert = CertificateParams::new(vec!["127.0.0.1".into()])
        .unwrap()
        .self_signed(&key)
        .unwrap();
    let der = CertificateDer::from(cert.der().to_vec());
    let provider = Arc::new(ring::default_provider());
    let server = ServerConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![der.clone()],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der())),
        )
        .unwrap();
    let mut roots = RootCertStore::empty();
    roots.add(der).unwrap();
    let client = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Certs {
        acceptor: TlsAcceptor::from(Arc::new(server)),
        trusted: TlsConnector::from(Arc::new(client)),
    }
}

/// An IMAP account on the test address.
pub fn account(address: &str) -> Account {
    let server = |port| Server {
        host: "127.0.0.1".into(),
        port,
        security: Security::Tls,
    };
    Account {
        address: address.into(),
        user: address.into(),
        auth: Auth::Password,
        protocol: Protocol::Imap,
        incoming: server(993),
        outgoing: server(465),
    }
}

pub fn login(secret: Secret) -> Login {
    Login {
        user: "ann@example.com".into(),
        secret,
    }
}

/// Serves one connection; the task fails on the first mismatch.
pub async fn serve(
    certs: &Certs,
    security: Security,
    steps: Vec<Step>,
) -> (Server, JoinHandle<Result<(), String>>) {
    serve_each(certs, security, vec![steps], None).await
}

/// Connection `i` runs script `i`; `accepted` hears each `i`.
pub async fn serve_each(
    certs: &Certs,
    security: Security,
    scripts: Vec<Vec<Step>>,
    accepted: Option<std::sync::mpsc::Sender<usize>>,
) -> (Server, JoinHandle<Result<(), String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let acceptor = certs.acceptor.clone();
    let task = tokio::spawn(async move {
        for (at, steps) in scripts.into_iter().enumerate() {
            let (tcp, _) = listener.accept().await.map_err(|err| err.to_string())?;
            if let Some(accepted) = &accepted {
                accepted.send(at).map_err(|err| err.to_string())?;
            }
            let io: Box<dyn Io> = match security {
                Security::Tls => Box::new(acceptor.accept(tcp).await.map_err(|e| e.to_string())?),
                Security::StartTls => Box::new(tcp),
            };
            run(io, &acceptor, steps).await?;
        }
        Ok(())
    });
    let server = Server {
        host: "127.0.0.1".into(),
        port,
        security,
    };
    (server, task)
}

async fn run(io: Box<dyn Io>, acceptor: &TlsAcceptor, steps: Vec<Step>) -> Result<(), String> {
    let mut io = BufReader::new(io);
    let mut tag = String::new();
    for step in steps {
        match step {
            Step::Send(text) => {
                let text = text.replace("{tag}", &tag).replace('\n', "\r\n");
                io.write_all(text.as_bytes())
                    .await
                    .map_err(|e| e.to_string())?;
                io.flush().await.map_err(|e| e.to_string())?;
            }
            Step::Expect(want) => {
                let got = line(&mut io).await?;
                let got = match want.strip_prefix("{tag} ") {
                    Some(_) => {
                        let (first, rest) = got.split_once(' ').unwrap_or((&got, ""));
                        tag = first.to_owned();
                        format!("{{tag}} {rest}")
                    }
                    None => got,
                };
                if got != want {
                    return Err(format!("expected {want:?}, got {got:?}"));
                }
            }
            Step::Data(want) => {
                let mut lines = Vec::new();
                loop {
                    let got = line(&mut io).await?;
                    if got == "." {
                        break;
                    }
                    lines.push(got);
                }
                let got = lines.join("\n");
                if got != want {
                    return Err(format!("expected data {want:?}, got {got:?}"));
                }
            }
            Step::AnyData => while line(&mut io).await? != "." {},
            Step::Prefix(want) => {
                let got = line(&mut io).await?;
                let got = match want.strip_prefix("{tag} ") {
                    Some(_) => {
                        let (first, rest) = got.split_once(' ').unwrap_or((&got, ""));
                        tag = first.to_owned();
                        format!("{{tag}} {rest}")
                    }
                    None => got,
                };
                if !got.starts_with(&want) {
                    return Err(format!("expected {want:?}..., got {got:?}"));
                }
            }
            Step::StartTls => {
                let tcp = io.into_inner();
                let tls = acceptor.accept(tcp).await.map_err(|e| e.to_string())?;
                io = BufReader::new(Box::new(tls));
            }
            Step::Run(act) => act(),
            Step::Hang => {
                tokio::spawn(async move {
                    let _held = io;
                    std::future::pending::<()>().await
                });
                return Ok(());
            }
        }
    }
    Ok(())
}

async fn line(io: &mut BufReader<Box<dyn Io>>) -> Result<String, String> {
    let mut line = String::new();
    io.read_line(&mut line).await.map_err(|e| e.to_string())?;
    match line.strip_suffix("\r\n") {
        Some(line) => Ok(line.to_owned()),
        None => Err(format!("client sent {line:?} without CRLF")),
    }
}
