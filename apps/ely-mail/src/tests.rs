//! Accounts in the window: add, check, keep, remove (C12.5).

use std::{
    collections::HashMap,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use ely_mail_engine::{
    Account, Auth, Backend, Checked, Protocol, Security, Server, Store, TlsConnector, Vault,
};
use gpui::{App, Entity, TestAppContext, VisualTestContext};

use crate::{
    adding::Step,
    kinds::Kind,
    mail::{Mail, Parts},
};

mod contacts;
mod controls;
mod drafts;
mod files;
mod flows;
mod leaving;
mod migrate;
mod organize;
mod outbox;
mod panes;
mod providers;
mod reading;
mod rules;
mod notices;
mod search;
mod side;
mod storage;
mod syncs;
mod threads;
mod whole;
mod writing;

/// A credential store that keeps or refuses.
#[derive(Clone, Default)]
pub(crate) struct Keys {
    pub kept: Arc<Mutex<HashMap<String, String>>>,
    pub refuse: Option<&'static str>,
    pub deletes: Arc<AtomicUsize>,
}

impl Backend for Keys {
    fn get(&self, name: &str) -> Result<Option<String>, String> {
        Ok(self.kept.lock().unwrap().get(name).cloned())
    }

    fn set(&self, name: &str, secret: &str) -> Result<(), String> {
        if let Some(reason) = self.refuse {
            return Err(reason.into());
        }
        self.kept.lock().unwrap().insert(name.into(), secret.into());
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        self.deletes.fetch_add(1, Ordering::SeqCst);
        if let Some(reason) = self.refuse {
            return Err(reason.into());
        }
        self.kept.lock().unwrap().remove(name);
        Ok(())
    }
}

pub(crate) fn settle(cx: &mut VisualTestContext, done: impl Fn(&App) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
        if cx.update(|_, app| done(app)) {
            return;
        }
    }
    panic!("the state never settled");
}

pub(crate) fn window<'a>(
    cx: &'a mut TestAppContext,
    dir: &Path,
    keys: Keys,
) -> (Entity<Mail>, &'a mut VisualTestContext) {
    let tls = ely_mail_engine::system_tls().map_err(|err| err.to_string());
    window_with(cx, dir, keys, tls)
}

pub(crate) fn window_with<'a>(
    cx: &'a mut TestAppContext,
    dir: &Path,
    keys: Keys,
    tls: Result<TlsConnector, String>,
) -> (Entity<Mail>, &'a mut VisualTestContext) {
    cx.executor().allow_parking();
    cx.update(ely_gpui_component::init_for_tests);
    cx.update(|cx| {
        ely_ui::settings::install_defaults(cx);
        ely_ui::i18n::install_as("en-US", cx);
        // As tool_root does: menus before the window.
        let menus = crate::menu::menus(cx);
        let menus = ely_ui::commands::menus(
            ely_platform::AppId::Mail,
            &ely_ui::commands::OPENING,
            false,
            menus,
            cx,
        );
        cx.set_menus(menus);
    });
    let parts = Parts {
        store: Store::open(dir).map_err(|err| err.to_string()),
        vault: Vault::new(Box::new(keys)),
        tls,
        copies: dir.join("opened"),
        settings: dir.join("settings.json"),
        open: std::sync::Arc::new(|path| {
            OPENED.lock().unwrap().push(path.to_path_buf());
            Ok(())
        }),
    };
    let (mail, cx) = cx.add_window_view(move |window, cx| Mail::new(parts, window, cx));
    settle(cx, |app| mail.read(app).accounts.is_some());
    (mail, cx)
}

/// Files the windows opened, in place of their apps.
pub(crate) static OPENED: std::sync::Mutex<Vec<std::path::PathBuf>> = std::sync::Mutex::new(Vec::new());

/// A port nothing listens on.
pub(crate) fn closed_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

/// On this machine's discard port, so syncs end at once.
pub(crate) fn account(address: &str) -> Account {
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
        incoming: server(9),
        outgoing: server(9),
    }
}

/// Opens the dialog on `kind`.
pub(crate) fn adding(mail: &Entity<Mail>, kind: Kind, cx: &mut VisualTestContext) {
    cx.update(|window, app| {
        mail.update(app, |mail, cx| {
            mail.start_adding(window, cx);
            mail.adding.as_mut().unwrap().set_kind(kind, cx);
        })
    });
}

pub(crate) fn type_in(
    mail: &Entity<Mail>,
    cx: &mut VisualTestContext,
    pick: fn(&crate::adding::Fields) -> &Entity<ely_gpui_component::forms::TextInput>,
    words: &str,
) {
    cx.update(|_, app| {
        let field = pick(&mail.read(app).adding.as_ref().unwrap().fields).clone();
        field.update(app, |field, cx| field.set_text(words.to_owned(), cx));
    });
    cx.run_until_parked();
}

/// A passed check, as servers would answer.
pub(crate) fn passed(
    mail: &Entity<Mail>,
    account: Account,
    kept: &str,
    cx: &mut VisualTestContext,
) {
    cx.update(|_, app| {
        mail.update(app, |mail, _| {
            mail.adding.as_mut().unwrap().step = Step::Checked {
                account,
                kept: kept.into(),
                checked: Checked {
                    incoming: Ok(()),
                    outgoing: Ok(()),
                },
            };
        })
    });
}

pub(crate) fn step_words(mail: &Entity<Mail>, cx: &mut VisualTestContext) -> String {
    cx.update(
        |_, app| match &mail.read(app).adding.as_ref().unwrap().step {
            Step::Failed(words) => words.to_string(),
            Step::Editing => "editing".into(),
            Step::Signing => "signing".into(),
            Step::Checking => "checking".into(),
            Step::Saving => "saving".into(),
            Step::Checked { .. } => "checked".into(),
        },
    )
}
