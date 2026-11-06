//! The kinds of account and their known servers.

use ely_mail_engine::{Auth, Protocol, Security, Server};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Gmail,
    Microsoft,
    ICloud,
    Qq,
    Mail163,
    Mail126,
    Imap,
    Pop,
}

pub const KINDS: [Kind; 8] = [
    Kind::Gmail,
    Kind::Microsoft,
    Kind::ICloud,
    Kind::Qq,
    Kind::Mail163,
    Kind::Mail126,
    Kind::Imap,
    Kind::Pop,
];

/// Where iCloud makes app-specific passwords.
pub const APPLE_ACCOUNT: &str = "https://account.apple.com";

/// Where Google makes app passwords.
pub const GOOGLE_APP_PASSWORDS: &str = "https://myaccount.google.com/apppasswords";

/// Where QQ Mail and NetEase make authorization codes.
pub const QQ_MAIL: &str = "https://mail.qq.com";
pub const MAIL_163: &str = "https://mail.163.com";
pub const MAIL_126: &str = "https://mail.126.com";

impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Kind::Gmail => "gmail",
            Kind::Microsoft => "microsoft",
            Kind::ICloud => "icloud",
            Kind::Qq => "qq",
            Kind::Mail163 => "163",
            Kind::Mail126 => "126",
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
            // QQ and NetEase: the address signs in, with a code.
            Kind::Qq => Some((
                Auth::Password,
                server("imap.qq.com", 993, Security::Tls),
                server("smtp.qq.com", 465, Security::Tls),
            )),
            Kind::Mail163 => Some((
                Auth::Password,
                server("imap.163.com", 993, Security::Tls),
                server("smtp.163.com", 465, Security::Tls),
            )),
            Kind::Mail126 => Some((
                Auth::Password,
                server("imap.126.com", 993, Security::Tls),
                server("smtp.126.com", 465, Security::Tls),
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
