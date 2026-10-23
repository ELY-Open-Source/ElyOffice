//! The kinds of account and their known servers.

use ely_mail_engine::{Auth, Protocol, Security, Server};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Gmail,
    Microsoft,
    ICloud,
    Imap,
    Pop,
}

pub const KINDS: [Kind; 5] = [
    Kind::Gmail,
    Kind::Microsoft,
    Kind::ICloud,
    Kind::Imap,
    Kind::Pop,
];

/// Where iCloud makes app-specific passwords.
pub const APPLE_ACCOUNT: &str = "https://account.apple.com";

/// Where Google makes app passwords.
pub const GOOGLE_APP_PASSWORDS: &str = "https://myaccount.google.com/apppasswords";

impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Kind::Gmail => "gmail",
            Kind::Microsoft => "microsoft",
            Kind::ICloud => "icloud",
            Kind::Imap => "imap",
            Kind::Pop => "pop",
        }
    }

    pub fn from_key(key: &str) -> Option<Kind> {
        KINDS.into_iter().find(|kind| kind.key() == key)
    }

    /// `keep` is POP3's server copy.
    pub fn protocol(self, keep: bool) -> Protocol {
        match self {
            Kind::Pop => Protocol::Pop { keep },
            _ => Protocol::Imap,
        }
    }

    /// The provider's servers and sign-in; None when typed in.
    pub fn known(self) -> Option<(Auth, Server, Server)> {
        let server = |host: &str, port, security| Server {
            host: host.into(),
            port,
            security,
        };
        match self {
            // Google's OAuth needs a security review; app passwords do not.
            Kind::Gmail => Some((
                Auth::Password,
                server("imap.gmail.com", 993, Security::Tls),
                server("smtp.gmail.com", 465, Security::Tls),
            )),
            Kind::Microsoft => Some((
                Auth::Microsoft,
                server("outlook.office365.com", 993, Security::Tls),
                server("smtp.office365.com", 587, Security::StartTls),
            )),
            // Apple: IMAP takes the full address too.
            Kind::ICloud => Some((
                Auth::Password,
                server("imap.mail.me.com", 993, Security::Tls),
                server("smtp.mail.me.com", 587, Security::StartTls),
            )),
            Kind::Imap | Kind::Pop => None,
        }
    }
}

/// The usual port for a protocol under a security.
pub fn port(protocol: Option<Protocol>, security: Security) -> u16 {
    match (protocol, security) {
        (Some(Protocol::Imap), Security::Tls) => 993,
        (Some(Protocol::Imap), Security::StartTls) => 143,
        (Some(Protocol::Pop { .. }), Security::Tls) => 995,
        (Some(Protocol::Pop { .. }), Security::StartTls) => 110,
        (None, Security::Tls) => 465,
        (None, Security::StartTls) => 587,
    }
}
