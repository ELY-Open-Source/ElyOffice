//! The title bar's button while a release is offered.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    primitives::IconName,
    shell::{Hosted, UpdateDialog, UpdateState},
    theme::ControlSize,
};
use gpui::{App, Context, Entity, WindowHandle, InteractiveElement as _, IntoElement, ParentElement as _, Render, Window, div};

use super::{Phase, Release, Updates};

impl Render for Updates {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some((_, phase)) = self.offer.as_ref().filter(|_| !self.dismissed) else {
            return div().into_any_element();
        };
        let label = match phase {
            Phase::Offered => crate::i18n::text(cx, "updates.button", &[]),
            Phase::Fetching(done) => format!("{}%", (done * 100.0).round()).into(),
            Phase::Fetched(_) => ely_gpui_component::i18n::text(cx, "update.restart", &[]),
        };
        let this = cx.entity();
        let button = Button::new("update-button", label)
            .icon(IconName::Download)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .on_click(move |_, _, cx| open(&this, cx));
        div().debug_selector(|| "update-button".into()).child(button).into_any_element()
    }
}

/// The offer, its download and restart, in a dialog.
pub(super) fn open(this: &Entity<Updates>, cx: &mut App) {
    // One dialog: a second click brings it forward.
    if let Some(shown) = this.read(cx).dialog.as_ref().map(|(shown, _)| *shown)
        && shown.update(cx, |_, window, _| window.activate_window()).is_ok()
    {
        return log::info!("updates: the open dialog comes forward");
    }
    let updates = this.read(cx);
    let Some((release, phase)) = updates.offer.clone() else {
        return log::info!("updates: the offer went before its dialog");
    };
    let (name, version) = (updates.app.name(), release.version.to_string());
    let state = state_of(&phase);
    let later = this.clone();
    let mut dialog = UpdateDialog::new(name, env!("CARGO_PKG_VERSION"), version.clone(), state).on_later(move |window, cx| {
        later.update(cx, |updates, cx| updates.later(cx));
        window.remove_window();
    });
    // A package manager installs it; Ely names the command.
    if let Some(command) = updates.command() {
        let args = [("app", name), ("version", version.as_str()), ("command", command.as_str())];
        dialog = dialog.note(crate::i18n::text(cx, "updates.package", &args));
    } else {
        let (download, restart) = (this.clone(), this.clone());
        dialog = dialog
            .on_download(move |window, cx| {
                download.update(cx, |updates, cx| updates.download(cx));
                window.remove_window();
            })
            .on_restart(move |_, cx| restart.update(cx, |updates, cx| updates.restart(cx)));
    }
    log::info!("updates: the dialog opens for {name} {version}");
    let handle = match dialog.open(cx) {
        Ok(handle) => handle,
        Err(error) => {
            log::error!("updates: the dialog did not open: {error}");
            return this.update(cx, |_, cx| cx.emit(super::UpdateNews::Failed(error.to_string())));
        }
    };
    let follow = cx.observe(this, move |this, cx| follow(&this, handle, &release, cx));
    this.update(cx, |updates, _| updates.dialog = Some((handle, follow)));
}

fn state_of(phase: &Phase) -> UpdateState {
    match phase {
        Phase::Offered => UpdateState::Available,
        Phase::Fetching(done) => UpdateState::Downloading(*done),
        Phase::Fetched(_) => UpdateState::Ready,
    }
}

/// The dialog shows the offer now, or closes with it.
fn follow(this: &Entity<Updates>, handle: WindowHandle<Hosted<UpdateDialog>>, shown: &Release, cx: &mut App) {
    let updates = this.read(cx);
    let state = updates
        .offer
        .as_ref()
        .filter(|(release, _)| !updates.dismissed && release == shown)
        .map(|(_, phase)| state_of(phase));
    let followed = handle.update(cx, |hosted, window, cx| match state {
        Some(state) => {
            hosted.content = hosted.content.clone().state(state);
            cx.notify();
        }
        None => window.remove_window(),
    });
    // A closed dialog stops following.
    if followed.is_err() || state.is_none() {
        log::info!("updates: the dialog no longer follows the offer");
        let this = this.clone();
        cx.defer(move |cx| {
            this.update(cx, |updates, _| {
                if updates.dialog.as_ref().is_some_and(|(shown, _)| *shown == handle) {
                    updates.dialog = None;
                }
            })
        });
    }
}

/// An offered release, for window tests.
#[cfg(test)]
pub(crate) fn offer_for_tests(cx: &mut gpui::App) {
    use gpui::AppContext as _;
    let release = ely_platform::update::Release {
        version: semver::Version::new(9, 0, 0),
        url: "https://example.test/9.dmg".into(),
        sha256: String::new(),
    };
    let updates = cx.new(|_| Updates {
        app: ely_platform::AppId::Mark,
        installed: ely_platform::install::Installed::Bundle(Default::default()),
        offer: Some((release, Phase::Offered)),
        dismissed: false,
        pending: None,
        checked: None,
        note: None,
        dialog: None,
    });
    cx.set_global(super::Updater(updates));
}
