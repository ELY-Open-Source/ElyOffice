//! A newer release: offered, downloaded, installed after quitting.

mod view;

use std::{
    io,
    path::{Path, PathBuf},
    process::Stdio,
    sync::atomic::AtomicBool,
};

use ely_platform::{
    AppId,
    install::{INSTALL, Installed},
    update::{self, Check, Release},
};
use futures::{StreamExt as _, channel::mpsc};
use gpui::{App, AppContext as _, Context, Entity, EventEmitter, Global};

/// The newest release's files; GitHub redirects, no API.
const LATEST: &str = "https://github.com/ELY-Open-Source/ElyOffice/releases/latest/download";

#[derive(Clone, Debug, PartialEq)]
enum Phase {
    Offered,
    Fetching(f32),
    Fetched(PathBuf),
}

/// The last check, as the settings page shows it.
#[derive(Clone, Debug, PartialEq)]
pub enum Checked {
    Asking,
    Latest,
    Newer(semver::Version),
    Failed(String),
}

pub struct Updates {
    app: AppId,
    installed: Installed,
    offer: Option<(Release, Phase)>,
    dismissed: bool,
    /// The download to install once the app has quit.
    pending: Option<PathBuf>,
    /// The last check's outcome.
    pub checked: Option<Checked>,
    /// Why the last install failed, for the window to say.
    pub note: Option<String>,
}

/// What an update says to the window.
pub enum UpdateNews {
    Failed(String),
    /// A check the user asked for found nothing newer.
    Latest,
}

impl EventEmitter<UpdateNews> for Updates {}

struct Updater(Entity<Updates>);

impl Global for Updater {}

/// The app's updater, if it runs from an install.
pub fn updater(cx: &App) -> Option<Entity<Updates>> {
    cx.try_global::<Updater>().map(|updater| updater.0.clone())
}

/// Checks now, as the user asked.
pub fn check_now(cx: &mut App) {
    if let Some(updates) = updater(cx) {
        updates.update(cx, |updates, cx| updates.check(true, cx));
    }
}

/// The quit an update asked for stopped: install nothing.
pub fn quit_stopped(cx: &mut App) {
    if let Some(updates) = updater(cx) {
        updates.update(cx, |updates, _| {
            if updates.pending.take().is_some() {
                log::info!("update: the quit stopped; nothing installs");
            }
        });
    }
}

/// Where a failed install leaves its reason.
fn note_path(app: AppId) -> io::Result<PathBuf> {
    Ok(ely_platform::data_dir(app).map_err(io::Error::other)?.join("updates/failed.txt"))
}

/// As the macOS installer: waits, installs, reopens; its exit code.
pub fn installer(app: AppId) -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    let [_, flag, dmg, bundle, pid] = args.as_slice() else {
        return None;
    };
    if flag != INSTALL {
        return None;
    }
    let Ok(pid) = pid.parse() else {
        eprintln!("update: {pid} is not a process number");
        return Some(2);
    };
    ely_platform::install::wait_for_exit(pid);
    let installed = ely_platform::install::install(Path::new(dmg), Path::new(bundle));
    if let Err(error) = &installed {
        // To the app's log; the reopened app shows it.
        eprintln!("update: the install failed: {error}");
        let note = note_path(app).and_then(|path| {
            std::fs::create_dir_all(path.parent().expect("the note's folder"))?;
            std::fs::write(path, error.to_string())
        });
        if let Err(lost) = note {
            eprintln!("update: the failure note was not written: {lost}");
        }
    }
    // Last: the note is written before the app reads it.
    if let Err(error) = ely_platform::install::reopen(Path::new(bundle)) {
        eprintln!("update: the app did not reopen: {error}");
        return Some(1);
    }
    Some(i32::from(installed.is_err()))
}

/// An installed app's updater; checks at launch if chosen.
pub(crate) fn install(app: AppId, cx: &mut App) {
    let installed = match ely_platform::install::running(app.slug()) {
        Ok(Some(installed)) => installed,
        Ok(None) => return log::info!("update: a dev build or a Nix install; no check"),
        Err(error) => return log::warn!("update: how this app was installed is unknown: {error}"),
    };
    log::info!("update: installed as {installed:?}");
    let note = note_path(app).ok().and_then(|path| {
        std::fs::read_to_string(&path)
            .ok()
            .inspect(|_| drop(std::fs::remove_file(&path)))
    });
    let updates = cx.new(|cx| {
        cx.on_app_quit(|this: &mut Updates, _| {
            this.start_installer();
            async {}
        })
        .detach();
        Updates {
            app,
            installed,
            offer: None,
            dismissed: false,
            pending: None,
            checked: None,
            note,
        }
    });
    cx.set_global(Updater(updates.clone()));
    // Off by default: no request unless the user chose it.
    if crate::settings::current(cx).check_updates {
        updates.update(cx, |updates, cx| updates.check(false, cx));
    } else {
        log::info!("update: no check at launch; it is off in settings");
    }
}

/// This build's manifest in the newest release, or the test's.
fn manifest(app: AppId) -> String {
    std::env::var("ELY_UPDATE_MANIFEST").unwrap_or_else(|_| latest(app))
}

fn latest(app: AppId) -> String {
    format!("{LATEST}/{}-{}.json", app.slug(), update::platform())
}

impl Updates {
    /// A package manager's command for an offered release.
    pub fn command(&self) -> Option<String> {
        let Installed::Package(package) = self.installed else {
            return None;
        };
        let (release, _) = self.offer.as_ref()?;
        Some(package.command(self.app.slug(), &release.version))
    }

    /// Asks for a newer release; `asked` reports every outcome.
    fn check(&mut self, asked: bool, cx: &mut Context<Self>) {
        // A download under way or done stays the offer.
        if matches!(
            self.offer,
            Some((_, Phase::Fetching(_) | Phase::Fetched(_)))
        ) {
            log::info!("update: a release is already on its way");
            if asked {
                self.dismissed = false;
                cx.notify();
            }
            return;
        }
        self.checked = Some(Checked::Asking);
        cx.notify();
        let app = self.app;
        let current =
            semver::Version::parse(env!("CARGO_PKG_VERSION")).expect("the build's version");
        // The lookup is a request too: off the window's thread.
        let checked = cx.background_spawn(async move { update::check(&current, &manifest(app)) });
        cx.spawn(async move |this, cx| {
            let checked = checked.await;
            this.update(cx, |this, cx| this.checked(checked, asked, cx)).ok();
        })
        .detach();
    }

    fn checked(&mut self, checked: anyhow::Result<Check>, asked: bool, cx: &mut Context<Self>) {
        self.checked = Some(match &checked {
            Ok(Check::Available(release)) => Checked::Newer(release.version.clone()),
            Ok(Check::Current) => Checked::Latest,
            Err(error) => Checked::Failed(format!("{error:#}")),
        });
        match checked {
            Ok(Check::Available(_))
                if matches!(
                    self.offer,
                    Some((_, Phase::Fetching(_) | Phase::Fetched(_)))
                ) => {}
            Ok(Check::Available(release)) => {
                (self.offer, self.dismissed) = (Some((release, Phase::Offered)), false);
            }
            Ok(Check::Current) if asked => cx.emit(UpdateNews::Latest),
            // At launch a failure only logs; offline is normal.
            Err(error) if asked => cx.emit(UpdateNews::Failed(format!("{error:#}"))),
            Err(error) => log::warn!("update: the launch check failed: {error:#}"),
            Ok(Check::Current) => log::info!("update: nothing newer at launch"),
        }
        cx.notify();
    }

    fn later(&mut self, cx: &mut Context<Self>) {
        self.dismissed = true;
        cx.notify();
    }

    fn download(&mut self, cx: &mut Context<Self>) {
        if matches!(self.installed, Installed::Package(_)) {
            return;
        }
        let Some((release, phase)) = self
            .offer
            .as_mut()
            .filter(|(_, phase)| *phase == Phase::Offered)
        else {
            return;
        };
        *phase = Phase::Fetching(0.0);
        cx.notify();
        let (release, app) = (release.clone(), self.app);
        let (sender, mut progress) = mpsc::unbounded();
        let fetching = release.clone();
        let fetched = cx.background_spawn(async move {
            let dir = ely_platform::data_dir(app)
                .map_err(|error| format!("{error:#}"))?
                .join("updates");
            if dir.exists() {
                std::fs::remove_dir_all(&dir).map_err(|error| error.to_string())?;
            }
            std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
            let cancel = AtomicBool::new(false);
            let mut tell = |done: u64, total: Option<u64>| {
                let part = total
                    .filter(|total| *total > 0)
                    .map_or(0.0, |total| done as f32 / total as f32);
                sender.unbounded_send(part).ok();
            };
            let file = update::download(&fetching, &dir, &cancel, update::IDLE_TIMEOUT, &mut tell)
                .map_err(|error| format!("{error:#}"))?
                .ok_or("the download stopped")?;
            update::verify_file(&file, &fetching.sha256).map_err(|error| format!("{error:#}"))?;
            Ok::<PathBuf, String>(file)
        });
        cx.spawn(async move |this, cx| {
            while let Some(part) = progress.next().await {
                this.update(cx, |this, cx| this.fetching(part, cx)).ok();
            }
            let fetched = fetched.await;
            this.update(cx, |this, cx| this.fetched(&release, fetched, cx))
                .ok();
        })
        .detach();
    }

    fn fetching(&mut self, part: f32, cx: &mut Context<Self>) {
        if let Some((_, phase @ Phase::Fetching(_))) = self.offer.as_mut() {
            *phase = Phase::Fetching(part);
            cx.notify();
        }
    }

    /// A finished download counts only for the release it fetched.
    fn fetched(
        &mut self,
        release: &Release,
        fetched: Result<PathBuf, String>,
        cx: &mut Context<Self>,
    ) {
        let Some((offered, phase)) = self
            .offer
            .as_mut()
            .filter(|(offered, _)| offered == release)
        else {
            return log::warn!("update: a download for another release was dropped");
        };
        match fetched {
            Ok(file) => *phase = Phase::Fetched(file),
            Err(error) => {
                *phase = Phase::Offered;
                cx.emit(UpdateNews::Failed(error));
            }
        }
        log::info!("update: {} is {:?}", offered.version, phase);
        cx.notify();
    }

    /// Quits as usual: unsaved work is asked about first.
    fn restart(&mut self, cx: &mut Context<Self>) {
        let Some((_, Phase::Fetched(file))) = self.offer.as_ref() else {
            return;
        };
        self.pending = Some(file.clone());
        crate::commands::quit(cx);
    }

    /// At the real quit: the installer waits for this process.
    fn start_installer(&mut self) {
        let Some(file) = self.pending.take() else {
            return;
        };
        let started = note_path(self.app).and_then(|note| {
            // Its words go to the app's log.
            let log = ely_platform::logging::path()
                .map(|path| std::fs::OpenOptions::new().append(true).open(path));
            let said = match log {
                Some(file) => Stdio::from(file?),
                None => Stdio::null(),
            };
            ely_platform::install::after_quit(&file, &self.installed, std::process::id(), &note, said)
        });
        match started {
            Ok(_) => log::info!("update: installs after this quit"),
            Err(error) => log::error!("update: the installer did not start: {error}"),
        }
    }
}

#[cfg(test)]
pub(crate) use view::offer_for_tests;

#[cfg(test)]
mod tests;
