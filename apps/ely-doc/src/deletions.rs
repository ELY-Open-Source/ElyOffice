//! Tracked deletions leave the text unless struck (#143).

use ely_office::prelude::*;
use ely_ui::settings::switches::{self, STRUCK};
use gpui::{App, Context};

use crate::docs::Docs;

/// What `.uno:HideTrackedDeletions` should report.
pub(crate) fn wanted(cx: &App) -> &'static str {
    if switches::on(STRUCK, cx) { "false" } else { "true" }
}

impl Docs {
    /// Tab `id` follows the deletions switch.
    pub(crate) fn follow_deletions(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(view) = self.tab(id).map(|tab| tab.view.clone()) else {
            return;
        };
        let want = wanted(cx);
        if view.read(cx).state(".uno:HideTrackedDeletions") == Some(want) {
            return;
        }
        log::info!("doc: tab {id} hides deletions {want}");
        if let Err(error) = view.update(cx, |view, _| view.run("track.hide", &[want])) {
            self.engine_failed(&error.to_string(), cx);
        }
    }
}
