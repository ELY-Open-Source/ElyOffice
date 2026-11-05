//! The window: accounts and the dialogs that change them.

use std::{
    collections::{HashMap, HashSet},
    future::Future,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use ely_gpui_component::primitives::Severity;
use ely_mail_engine::{
    Account, AccountId, Auth, DraftId, Filter, MessageId, Queued, Shelved, Store, TlsConnector,
    Vault, oauth,
};
use ely_platform::AppId;
use ely_ui::Root;
use gpui::{App, AppContext as _, Context, Entity, FocusHandle, SharedString, WeakEntity, Window};
use tokio::runtime::Runtime;

use crate::{
    adding::Adding,
    boxes::{Listing, Open, Unread},
    drafts::Waiting,
    identity::Editing,
    organize::Erasing,
    outbox::Runner,
    reading::Reading,
    removing::Removing,
    seeking::Seeking,
    side::Side,
    syncing::Syncs,
};

/// Opens a file in its app.
pub type Opener = Arc<dyn Fn(&Path) -> std::io::Result<()> + Send + Sync>;

/// The OAuth server for a way of signing in.
pub type Providers = Arc<dyn Fn(Auth) -> Option<ely_mail_engine::Result<oauth::Provider>> + Send + Sync>;

/// What the engine runs on and keeps to.
pub struct Parts {
    pub store: Result<Store, String>,
    pub vault: Vault,
    pub tls: Result<TlsConnector, String>,
    pub providers: Providers,
    /// Working copies of opened files go here.
    pub copies: PathBuf,
    /// The app's settings file.
    pub settings: PathBuf,
    pub open: Opener,
}

impl Parts {
    pub fn system(dir: &Path) -> Self {
        Self {
            store: Store::open(dir).map_err(|err| err.to_string()),
            vault: Vault::system(),
            tls: ely_mail_engine::system_tls().map_err(|err| err.to_string()),
            providers: Arc::new(Auth::provider),
            copies: dir.join("opened"),
            settings: dir.join("settings.json"),
            open: Arc::new(ely_platform::apps::open_file),
        }
    }
}

pub struct Mail {
    pub store: Result<Arc<Mutex<Store>>, String>,
    pub read_only: Option<String>,
    pub vault: Arc<Vault>,
    pub tls: Result<TlsConnector, String>,
    pub providers: Providers,
    pub(crate) runtime: Arc<Runtime>,
    /// None until read.
    pub accounts: Option<Result<Vec<(AccountId, Account)>, String>>,
    /// Accounts the side folds away.
    pub folded: HashSet<AccountId>,
    /// Remove acts on its account.
    pub open: Open,
    pub filter: Filter,
    pub seeking: Seeking,
    pub listing: Listing,
    pub reading: Reading,
    pub unread: Unread,
    pub adding: Option<Adding>,
    pub removing: Option<Removing>,
    /// The open account's sender name and signature.
    pub identity: Option<Editing>,
    /// The open account's rules.
    pub ruling: Option<crate::rules::Ruling>,
    /// The contacts page's list.
    pub people: Option<crate::contacts::People>,
    /// An imported folder whose removal waits for an answer.
    pub unimporting: Option<crate::migrate::Unimporting>,
    /// Mail's storage rows in settings.
    pub cache: crate::storage::Cache,
    pub cap: crate::capacity::Cap,
    pub settings: PathBuf,
    pub syncs: Syncs,
    /// Numbers every sync, even across forgotten accounts.
    pub runs: u64,
    /// Until resumed, no account syncs.
    pub paused: bool,
    /// Numbers every request across dialogs.
    pub asked: u64,
    /// Server copies under way, and the next asked.
    pub pushing: HashMap<DraftId, Option<Waiting>>,
    /// Drafts read to reopen; their windows follow.
    pub reopening: HashSet<DraftId>,
    /// Deleted while reopening: no window follows.
    pub unwanted: HashSet<DraftId>,
    /// The drafts listed, when Drafts is open.
    pub shelved: Vec<Shelved>,
    /// A draft asked to be deleted.
    pub deleting: Option<DraftId>,
    /// The open account's drafts go to its server.
    pub drafts_up: Option<(AccountId, bool)>,
    /// The open account's Sent copy choice; None follows the provider.
    pub sent_copy: Option<(AccountId, Option<bool>)>,
    pub runner: Runner,
    /// The messages listed, when Outbox is open.
    pub queued: Vec<Queued>,
    /// Outbox work a quit would leave undone.
    pub pending: usize,
    /// Messages asked to go for good.
    pub erasing: Option<Erasing>,
    /// Messages coming down whole for their files.
    pub downloading: HashSet<MessageId>,
    pub copies: PathBuf,
    pub open_copy: Opener,
    /// The quit question is showing.
    pub quitting: bool,
    /// Asked once, the quit goes on.
    pub quit_anyway: bool,
    pub focus: FocusHandle,
    pub root: WeakEntity<Root>,
    pub side: Entity<Side>,
}

pub fn text(cx: &App, key: &str, args: &[(&str, &str)]) -> SharedString {
    ely_ui::i18n::text(cx, key, args)
}

pub fn open(app: AppId, window: &mut Window, cx: &mut App) -> Entity<Root> {
    let dir = match ely_platform::data_dir(app) {
        Ok(dir) => dir,
        Err(error) => ely_platform::startup::fail(app, &format!("{error:#}")),
    };
    let parts = Parts::system(&dir);
    let mail = cx.new(|cx| Mail::new(parts, window, cx));
    crate::storage::install(mail.downgrade(), cx);
    let root = attach(app, &mail, false, window, cx);
    // Message files open to read; others go into a draft.
    let files = ely_ui::launch_files(cx);
    mail.update(cx, |mail, cx| mail.take_files(files, cx));
    let handed = mail.downgrade();
    ely_ui::on_open(cx, move |path, cx| {
        handed.update(cx, |mail, cx| mail.take_files(vec![path], cx)).ok();
    });
    // Closing the window leaves sync and the Outbox running.
    let kept = mail.clone();
    let tray = ely_ui::tray::keep(app, TRAY_PNG, "envelope", move |window, cx| attach(app, &kept, true, window, cx), cx);
    if let Err(error) = tray {
        log::warn!("mail: no tray, so closing the window quits: {error:#}");
    }
    root
}

const TRAY_PNG: &[u8] = include_bytes!("../../../assets/icons/ely-mail.png");

/// The window's root around `mail`; `again` from the tray.
pub(crate) fn attach(app: AppId, mail: &Entity<Mail>, again: bool, window: &mut Window, cx: &mut App) -> Entity<Root> {
    let root = ely_ui::tool_root(app, mail.clone().into(), crate::menu::menus(cx), window, cx);
    mail.update(cx, |mail, cx| {
        if again {
            mail.rehome(window, cx);
        }
        mail.root = root.downgrade();
        window.focus(&mail.focus, cx);
    });
    crate::writing::guard_leaving(mail.downgrade(), window, cx);
    root
}

impl Mail {
    pub fn new(parts: Parts, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("mail")
            .enable_all()
            .build()
            .unwrap_or_else(|error| {
                ely_platform::startup::fail(AppId::Mail, &format!("no mail runtime: {error}"))
            });
        // A send cut off last time is unknown now.
        let store = parts.store.and_then(|store| match store.read_only() {
            Some(_) => Ok(store),
            None => match store.recover_outbox() {
                Ok(_) => Ok(store),
                Err(error) => Err(format!("the Outbox was not checked: {error}")),
            },
        });
        let read_only = match &store {
            Ok(store) => store.read_only().map(str::to_owned),
            Err(error) => {
                log::error!("mail: the store did not open: {error}");
                None
            }
        };
        if let Err(error) = &parts.tls {
            log::error!("mail: no system TLS: {error}");
        }
        let this = cx.weak_entity();
        let mut mail = Mail {
            store: store.map(|store| Arc::new(Mutex::new(store))),
            read_only,
            vault: Arc::new(parts.vault),
            tls: parts.tls,
            providers: parts.providers,
            runtime: Arc::new(runtime),
            accounts: None,
            folded: HashSet::new(),
            open: Open::Inboxes,
            filter: Filter::default(),
            seeking: Seeking::new(window, cx),
            listing: Listing::default(),
            reading: Reading::default(),
            unread: Unread::new(),
            adding: None,
            removing: None,
            identity: None,
            ruling: None,
            people: None,
            unimporting: None,
            cache: Default::default(),
            cap: Default::default(),
            settings: parts.settings,
            syncs: Syncs::new(),
            runs: 0,
            paused: false,
            asked: 0,
            pushing: HashMap::new(),
            reopening: HashSet::new(),
            unwanted: HashSet::new(),
            shelved: Vec::new(),
            deleting: None,
            drafts_up: None,
            sent_copy: None,
            runner: Runner::default(),
            queued: Vec::new(),
            pending: 0,
            erasing: None,
            downloading: HashSet::new(),
            copies: parts.copies,
            open_copy: parts.open,
            quitting: false,
            quit_anyway: false,
            focus: cx.focus_handle(),
            root: WeakEntity::new_invalid(),
            side: cx.new(|_| Side::new(this)),
        };
        mail.read_accounts(cx);
        mail.read_cap(cx);
        Self::sync_on_a_timer(cx);
        Self::answer_notices(cx);
        mail
    }

    pub fn read_accounts(&mut self, cx: &mut Context<Self>) {
        self.with_store(
            cx,
            |store| Ok((store.accounts()?, store.folded()?)),
            |mail, read, cx| {
                let first = mail.accounts.is_none();
                let read = read.map(|(accounts, folded)| {
                    mail.folded = folded.into_iter().collect();
                    accounts
                });
                if let Ok(accounts) = &read {
                    let still = |id| accounts.iter().any(|(each, _)| *each == id);
                    if mail.open.account().is_some_and(|id| !still(id)) {
                        mail.open = Open::Inboxes;
                    }
                }
                mail.accounts = Some(read);
                if first {
                    mail.sync_all(cx);
                }
                mail.read_mail(cx);
            },
        );
    }

    pub fn accounts(&self) -> &[(AccountId, Account)] {
        match &self.accounts {
            Some(Ok(accounts)) => accounts,
            _ => &[],
        }
    }

    /// Redraws the window and its account list.
    pub fn notify(&self, cx: &mut Context<Self>) {
        self.side.update(cx, |_, cx| cx.notify());
        cx.notify();
    }

    /// Store work off the window's thread.
    pub fn with_store<R: Send + 'static>(
        &self,
        cx: &mut Context<Self>,
        work: impl FnOnce(&mut Store) -> ely_mail_engine::Result<R> + Send + 'static,
        then: impl FnOnce(&mut Mail, Result<R, String>, &mut Context<Mail>) + 'static,
    ) {
        let store = match &self.store {
            Ok(store) => store.clone(),
            Err(error) => {
                self.failed("mail.error.store", error, cx);
                return;
            }
        };
        let read = cx.background_spawn(async move {
            let mut store = store
                .lock()
                .map_err(|_| "a store task panicked".to_owned())?;
            work(&mut store).map_err(|err| err.to_string())
        });
        cx.spawn(async move |this, cx| {
            let read = read.await;
            this.update(cx, |mail, cx| {
                then(mail, read, cx);
                mail.notify(cx);
            })
            .ok();
        })
        .detach();
    }

    /// Network work on the mail runtime; abort stops it.
    pub fn on_runtime<R: Send + 'static>(
        &self,
        cx: &mut Context<Self>,
        work: impl Future<Output = R> + Send + 'static,
        then: impl FnOnce(&mut Mail, Result<R, String>, &mut Context<Mail>) + 'static,
    ) -> tokio::task::AbortHandle {
        let task = self.runtime.spawn(work);
        let abort = task.abort_handle();
        cx.spawn(async move |this, cx| {
            let done = task.await;
            if done.as_ref().is_err_and(|err| err.is_cancelled()) {
                return;
            }
            let done = done.map_err(|err| format!("a mail task failed: {err}"));
            this.update(cx, |mail, cx| {
                then(mail, done, cx);
                mail.notify(cx);
            })
            .ok();
        })
        .detach();
        abort
    }

    /// Not logged: the words may name an address.
    pub fn toast(&self, text: SharedString, severity: Severity, cx: &mut App) {
        match self.root.upgrade() {
            Some(root) => root.update(cx, |root, cx| root.toast(text, severity, cx)),
            None => log::info!("mail: the window is closed, so a note is not shown"),
        }
    }

    /// A reopened window: inputs of the closed one go.
    pub(crate) fn rehome(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        log::info!("mail: the window opened again");
        self.adding = None;
        self.seeking.rehome(window, cx);
    }

    pub fn failed(&self, key: &str, error: &str, cx: &mut App) {
        log::warn!("mail: {key}");
        let words = text(cx, key, &[("error", error)]);
        self.toast(words, Severity::Danger, cx);
    }
}
