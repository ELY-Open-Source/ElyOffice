use std::{fmt, io};

pub type Result<T> = std::result::Result<T, Error>;

/// Why a mail server conversation failed; server words kept.
#[derive(Debug)]
pub enum Error {
    /// No TCP connection: name lookup, refusal, reset.
    Connect(io::Error),
    /// TLS handshake or certificate check failed.
    Tls(String),
    /// The server refused the credentials.
    Rejected(String),
    /// The server refused a command after login.
    Server(String),
    /// The server said something the client cannot read.
    Protocol(String),
    /// The server did not answer in time.
    Timeout(crate::Stage),
    /// The message went out; its acceptance is unknown.
    Unsure(String),
    /// An address or login the protocol cannot carry.
    Invalid(String),
    /// A message that cannot be read or written.
    Mime(String),
    /// The local store failed: database, disk or damage.
    Store(String),
    /// An account with this address is already kept.
    Taken(String),
    /// The store refuses writes; the reason says why.
    ReadOnly(String),
    /// The system credential store failed.
    Vault(String),
    /// This build has no client ID for the provider.
    Unavailable(&'static str),
    /// Sign-in did not finish; the reason says why.
    SignIn(String),
    /// The provider revoked the refresh token.
    SignInAgain(String),
    /// A remote picture did not come.
    Fetch(String),
    /// The server lacks an extension the step needs.
    Unsupported(&'static str),
    /// Larger than the server takes, in bytes.
    TooLarge { size: u64, limit: u64 },
    /// Deleted here; a late download is not kept.
    Deleted,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect(err) => write!(f, "cannot connect: {err}"),
            Self::Tls(text) => write!(f, "TLS failed: {text}"),
            Self::Rejected(text) => write!(f, "login refused: {text}"),
            Self::Server(text) => write!(f, "server refused: {text}"),
            Self::Protocol(text) => write!(f, "unreadable reply: {text}"),
            Self::Timeout(step) => write!(f, "no answer while {step}"),
            Self::Unsure(text) => write!(f, "the server may have accepted it: {text}"),
            Self::Invalid(text) => write!(f, "cannot be sent: {text}"),
            Self::Mime(text) => write!(f, "unreadable message: {text}"),
            Self::Store(text) => write!(f, "local store failed: {text}"),
            Self::Taken(address) => write!(f, "{address} is already added"),
            Self::ReadOnly(text) => write!(f, "local store is read-only: {text}"),
            Self::Vault(text) => write!(f, "credential store failed: {text}"),
            Self::Unavailable(name) => write!(f, "{name} sign-in is not in this build"),
            Self::SignIn(text) => write!(f, "sign-in failed: {text}"),
            Self::SignInAgain(text) => write!(f, "sign in again: {text}"),
            Self::Fetch(text) => write!(f, "picture not fetched: {text}"),
            Self::Unsupported(name) => write!(f, "the server does not support {name}"),
            Self::TooLarge { size, limit } => {
                write!(f, "the message is {size} bytes; the server takes {limit}")
            }
            Self::Deleted => write!(f, "the message was deleted here"),
        }
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Self::Store(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::Store(format!("a stored field: {err}"))
    }
}
