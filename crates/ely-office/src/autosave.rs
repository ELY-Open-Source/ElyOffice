//! Ordinary files save themselves when chosen (§6.2).

use std::time::Duration;

use ely_files::SaveState;
use gpui::{AsyncApp, Context, WeakEntity};

use crate::{Engine, Host, Recovering as _, Saving as _, Tabs as _, in_window};

/// How long typing rests before a save.
const QUIET: Duration = Duration::from_secs(2);

pub trait Autosaving: Host {
    /// The window's beat; recovery points wait on it too.
    fn start_autosave(cx: &mut Context<Self>) {
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            loop {
                cx.background_executor().timer(QUIET).await;
                if in_window(&this, cx, |host, window, cx| host.beat(window, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    /// Rested tabs get their owed points, then autosaves.
    fn beat(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        let rested = self.rested(cx);
        self.points_after_pause(&rested, cx);
        self.autosave(rested, window, cx);
    }

    /// Settled tabs whose edits held still a whole beat.
    fn rested(&mut self, cx: &mut Context<Self>) -> Vec<u64> {
        let edits: Vec<(u64, u64)> = self
            .core()
            .tabs
            .iter()
            .filter(|tab| Self::Engine::settled(tab.view.read(cx)))
            .map(|tab| (tab.id, Self::Engine::edited(tab.view.read(cx))))
            .collect();
        let quiet = &self.core().quiet;
        let rested = edits.iter().filter(|(id, edited)| quiet.get(id) == Some(edited)).map(|(id, _)| *id).collect();
        self.core_mut().quiet = edits.into_iter().collect();
        rested
    }

    /// Saves each eligible tab whose edits rested a whole beat.
    fn autosave(&mut self, rested: Vec<u64>, window: &mut gpui::Window, cx: &mut Context<Self>) {
        let on = ely_ui::settings::switches::on(ely_ui::settings::switches::AUTOSAVE, cx);
        let core = self.core();
        if !on || core.stopped.is_some() || core.dialog.is_some() {
            return;
        }
        let due: Vec<u64> = rested
            .into_iter()
            .filter(|id| {
                self.tab(*id).is_some_and(|tab| {
                    let ours = tab.session.path().and_then(|path| path.extension()).and_then(|ext| ext.to_str()).is_some_and(
                        |ext| Self::KIND.save_as.iter().any(|format| format.eq_ignore_ascii_case(ext)),
                    );
                    ours && tab.risk.is_none() && !tab.exporting && tab.session.state() == SaveState::Dirty
                })
            })
            .collect();
        // Rest stays: recovery reads it; saved tabs are not dirty.
        for id in due {
            log::info!("office: autosaving tab {id}");
            // A whole beat rested: the pause is had.
            self.save_paused(id, None, window, cx);
        }
    }
}

impl<H: Host> Autosaving for H {}
