//! A save whose file was moved or deleted outside.

use std::path::Path;

use ely_gpui_component::primitives::Severity;
use gpui::Context;

use super::Saving;
use crate::{After, Tabs as _, text};

/// A program hears at once; a person picks a place.
pub(super) fn gone<H: Saving>(host: &mut H, id: u64, path: &Path, then: Option<After>, cx: &mut Context<H>) {
    log::warn!("office: {} is gone; tab {id} cannot save in place", path.display());
    let Some(tab) = host.tab_mut(id) else { return };
    let name = tab.name.clone();
    // Close still asks; autosave stops prompting.
    tab.session.save_failed();
    cx.notify();
    // Only a program's save waits here.
    if host.core().saving.iter().any(|(tab, _)| *tab == id) {
        let why = format!("{} was moved or deleted; save it under another name", path.display());
        return host.core_mut().answer_saves(id, Err(why));
    }
    let words = text(cx, "office.gone", &[("name", &name)]);
    host.toast(words, Severity::Warning, cx);
    host.save_as(id, then, cx);
}
