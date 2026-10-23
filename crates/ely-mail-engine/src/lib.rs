//! Mail protocols, sync and local storage for ElyMail.

mod account;
pub mod compose;
pub mod drafts;
mod error;
pub mod fetch;
mod html;
mod imap;
mod mime;
mod net;
pub mod oauth;
pub mod page;
mod pop;
mod proxy;
pub mod push;
pub mod sent;
mod smtp;
mod store;
pub mod mbox;
pub mod sync;
pub mod vcard;
pub mod whole;
mod vault;
mod wire;

pub use account::{Account, Auth, Checked, Protocol};
pub use error::{Error, Result};
pub use imap::{Folder, Got, Imap, Opened, Role};
pub use mime::{Built, Draft, Mail, Mailbox, Part, Shown, build, read, without_bcc};
pub use net::{Login, Secret, Security, Server, system_tls};
pub use pop::Pop;
pub use smtp::Smtp;
pub use store::{
    AccountId, Action, Arrivals, Cached, Change, Contact, ContactId, DraftId, Fetched, Field, Filed, Filing, Filter, Found,
    Going, Handed, Held, Identity, Imported, Key, Landing, Left, Letter, LocalFolder, MailboxId, Match,
    MessageId, OutboxId, Packed, Queued, Remote, Rule, RuleId, Scope, Search, Shelved, Sizes, Store,
    Stored, Stowed, Summary, Synced, Unpacked, Unsent,
};
pub use tokio_rustls::TlsConnector;
pub use vault::{Backend, Kept, Keyring, Vault};
