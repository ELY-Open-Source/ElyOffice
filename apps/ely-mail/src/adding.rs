//! The add-account dialog's fields and steps (MAIL-01, 02).

use ely_gpui_component::forms::{InputEvent, TextInput};
use ely_mail_engine::{Account, AccountId, Auth, Checked, Protocol, Security, Server};
use gpui::{App, AppContext as _, Context, Entity, SharedString, Subscription, Window};
use tokio::task::AbortHandle;

use crate::{
    kinds::{self, Kind},
    mail::{Mail, text},
};

pub struct Fields {
    pub address: Entity<TextInput>,
    pub user: Entity<TextInput>,
    pub password: Entity<TextInput>,
    pub incoming: Entity<TextInput>,
    pub incoming_port: Entity<TextInput>,
    pub outgoing: Entity<TextInput>,
    pub outgoing_port: Entity<TextInput>,
}

pub enum Step {
    Editing,
    /// The browser holds the provider's page.
    Signing,
    Checking,
    /// `kept` goes to the vault: a password or refresh token.
    Checked {
        account: Account,
        kept: String,
        checked: Checked,
    },
    Failed(SharedString),
    Saving,
}

pub struct Adding {
    pub kind: Kind,
    pub fields: Fields,
    pub incoming_security: Security,
    pub outgoing_security: Security,
    /// Typed-in accounts choose; providers fix it.
    pub auth: Auth,
    /// POP3 leaves a copy on the server (§12.3).
    pub keep: bool,
    pub step: Step,
    /// The ports last filled in; any other was typed.
    filled: [String; 2],
    /// The request awaited; any other answer is dropped.
    pub asked: Option<u64>,
    pub task: Option<AbortHandle>,
    /// The account whose refused token this replaces.
    pub again: Option<AccountId>,
    _edits: Vec<Subscription>,
}

impl Drop for Adding {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Adding {
    pub fn new(kind: Kind, window: &mut Window, cx: &mut Context<Mail>) -> Self {
        let mut field = |key: &str, masked: bool, cx: &mut Context<Mail>| {
            let words = text(cx, key, &[]);
            cx.new(|cx| {
                let input = TextInput::new(window, cx)
                    .placeholder(words.clone())
                    .label(words);
                if masked { input.masked() } else { input }
            })
        };
        let address = field("mail.add.address", false, cx);
        let user = field("mail.add.user", false, cx);
        let password = field("mail.add.password", true, cx);
        let incoming = field("mail.add.incoming", false, cx);
        let incoming_port = field("mail.add.port", false, cx);
        let outgoing = field("mail.add.outgoing", false, cx);
        let outgoing_port = field("mail.add.port", false, cx);
        let fields = Fields {
            address,
            user,
            password,
            incoming,
            incoming_port,
            outgoing,
            outgoing_port,
        };
        let edits = [
            &fields.address,
            &fields.user,
            &fields.password,
            &fields.incoming,
            &fields.incoming_port,
            &fields.outgoing,
            &fields.outgoing_port,
        ]
        .into_iter()
        .map(|field| {
            cx.subscribe(field, |mail: &mut Mail, _, event: &InputEvent, cx| {
                if *event == InputEvent::Changed {
                    mail.edited(cx);
                }
            })
        })
        .collect();
        let mut adding = Adding {
            kind,
            fields,
            incoming_security: Security::Tls,
            outgoing_security: Security::StartTls,
            auth: Auth::Password,
            keep: true,
            step: Step::Editing,
            filled: Default::default(),
            asked: None,
            task: None,
            again: None,
            _edits: edits,
        };
        adding.set_ports(cx);
        adding
    }

    /// Signs `account` in again, its servers as kept.
    pub fn again(id: AccountId, account: &Account, window: &mut Window, cx: &mut Context<Mail>) -> Self {
        let kind = match account.protocol {
            Protocol::Imap => Kind::Imap,
            Protocol::Pop { .. } => Kind::Pop,
        };
        let mut adding = Self::new(kind, window, cx);
        adding.again = Some(id);
        adding.auth = account.auth;
        adding.incoming_security = account.incoming.security;
        adding.outgoing_security = account.outgoing.security;
        if let Protocol::Pop { keep } = account.protocol {
            adding.keep = keep;
        }
        let fields = &adding.fields;
        let values = [
            (&fields.address, account.address.clone()),
            (&fields.incoming, account.incoming.host.clone()),
            (&fields.incoming_port, account.incoming.port.to_string()),
            (&fields.outgoing, account.outgoing.host.clone()),
            (&fields.outgoing_port, account.outgoing.port.to_string()),
        ];
        for (field, value) in values {
            field.update(cx, |field, cx| field.set_text(value, cx));
        }
        adding
    }

    pub fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.asked = None;
    }

    /// A new kind keeps what was typed.
    pub fn set_kind(&mut self, kind: Kind, cx: &mut App) {
        self.stop();
        self.kind = kind;
        self.step = Step::Editing;
        self.set_ports(cx);
    }

    pub fn set_security(&mut self, outgoing: bool, security: Security, cx: &mut App) {
        self.stop();
        self.step = Step::Editing;
        if outgoing {
            self.outgoing_security = security;
        } else {
            self.incoming_security = security;
        }
        self.set_ports(cx);
    }

    pub fn set_keep(&mut self, keep: bool) {
        self.stop();
        self.step = Step::Editing;
        self.keep = keep;
    }

    /// The usual ports, unless one was typed.
    fn set_ports(&mut self, cx: &mut App) {
        let ports = [
            kinds::port(Some(self.kind.protocol(self.keep)), self.incoming_security),
            kinds::port(None, self.outgoing_security),
        ];
        let fields = [&self.fields.incoming_port, &self.fields.outgoing_port];
        for ((field, port), filled) in fields.into_iter().zip(ports).zip(&mut self.filled) {
            let typed = field.read(cx).text().trim().to_owned();
            if typed.is_empty() || typed == *filled {
                *filled = port.to_string();
                field.update(cx, |field, cx| field.set_text(filled.clone(), cx));
            }
        }
    }

    /// The sign-in this kind uses.
    pub fn auth(&self) -> Auth {
        match self.kind.known() {
            Some((auth, ..)) => auth,
            None => self.auth,
        }
    }

    fn read(&self, field: &Entity<TextInput>, cx: &App) -> String {
        field.read(cx).text().trim().to_owned()
    }

    /// The servers as known or typed.
    pub fn servers(&self, cx: &App) -> Result<(Server, Server), SharedString> {
        if let Some((_, incoming, outgoing)) = self.kind.known() {
            return Ok((incoming, outgoing));
        }
        let server = |host: &Entity<TextInput>, port: &Entity<TextInput>, security| {
            let host = self.read(host, cx);
            if host.is_empty() {
                return Err(text(cx, "mail.add.need_host", &[]));
            }
            let port = self.read(port, cx);
            let port = port
                .parse()
                .ok()
                .filter(|port| *port > 0)
                .ok_or_else(|| text(cx, "mail.add.bad_port", &[("port", &port)]))?;
            Ok(Server {
                host,
                port,
                security,
            })
        };
        Ok((
            server(
                &self.fields.incoming,
                &self.fields.incoming_port,
                self.incoming_security,
            )?,
            server(
                &self.fields.outgoing,
                &self.fields.outgoing_port,
                self.outgoing_security,
            )?,
        ))
    }

    /// The account the fields name; `signed` is an OAuth address.
    pub fn account(&self, signed: Option<&str>, cx: &App) -> Result<Account, SharedString> {
        let address = match signed {
            Some(address) => address.to_owned(),
            None => self.read(&self.fields.address, cx),
        };
        if address.is_empty() {
            return Err(text(cx, "mail.add.need_address", &[]));
        }
        let user = match (signed, self.kind) {
            (None, Kind::Imap | Kind::Pop) => self.read(&self.fields.user, cx),
            _ => address.clone(),
        };
        if user.is_empty() {
            return Err(text(cx, "mail.add.need_user", &[]));
        }
        let (incoming, outgoing) = self.servers(cx)?;
        let account = Account {
            address,
            user,
            auth: self.auth(),
            protocol: self.kind.protocol(self.keep),
            incoming,
            outgoing,
        };
        account
            .valid()
            .map_err(|err| text(cx, "mail.add.invalid", &[("error", &err.to_string())]))?;
        Ok(account)
    }

    pub fn password(&self, cx: &App) -> Result<String, SharedString> {
        let password = self.fields.password.read(cx).text().to_owned();
        if password.is_empty() {
            return Err(text(cx, "mail.add.need_password", &[]));
        }
        Ok(password)
    }

    pub fn busy(&self) -> bool {
        matches!(self.step, Step::Signing | Step::Checking | Step::Saving)
    }
}
