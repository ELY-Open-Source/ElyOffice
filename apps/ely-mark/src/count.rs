//! Words and characters in the status bar (#30).

use ely_gpui_component::shell::StatusBarItem;
use ely_office::prelude::*;
use std::sync::Arc;

use gpui::{App, Context};
use unicode_segmentation::UnicodeSegmentation as _;

use crate::desk::Desk;

/// Words, an ideograph each; characters as seen, marks included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counts {
    pub words: u64,
    pub characters: u64,
}

pub fn count(text: &str) -> Counts {
    Counts {
        words: text.unicode_words().count() as u64,
        characters: text.graphemes(true).count() as u64,
    }
}

impl Desk {
    /// Counts tab `id` off the window when its text changed.
    pub(crate) fn recount(&mut self, id: u64, changed: u64, cx: &mut Context<Self>) {
        let Some(tab) = self.tab_mut(id).filter(|tab| tab.extra.counting != Some(changed)) else {
            return;
        };
        tab.extra.counting = Some(changed);
        let text = tab.doc.lock().expect("the text lock").current.clone();
        cx.spawn(async move |desk, cx| {
            let counts = cx.background_executor().spawn(async move { count(&text) }).await;
            let landed = desk.update(cx, |desk, cx| {
                let tab = desk.tab_mut(id).filter(|tab| tab.extra.counting == Some(changed));
                if let Some(tab) = tab {
                    tab.extra.counts = Some(counts);
                    cx.notify();
                }
            });
            if landed.is_err() {
                log::info!("mark: the window closed before a count");
            }
        })
        .detach();
    }

    /// Counts a new selection off the window; drawing only reads.
    pub(crate) fn count_selection(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.core.active else {
            return;
        };
        let Some(view) = self.active_view() else {
            return;
        };
        let selected = view.read(cx).document().read(cx).selected(cx);
        let Some(tab) = self.tab_mut(id) else {
            return;
        };
        if selected.is_empty() {
            tab.extra.selection = None;
            return;
        }
        if tab.extra.selection.as_ref().is_some_and(|(held, _)| **held == *selected) {
            return;
        }
        let held: Arc<str> = selected.into();
        tab.extra.selection = Some((held.clone(), None));
        cx.spawn(async move |desk, cx| {
            let text = held.clone();
            let words = cx.background_executor().spawn(async move { count(&text).words }).await;
            let landed = desk.update(cx, |desk, cx| desk.land_selection(id, &held, words, cx));
            if landed.is_err() {
                log::info!("mark: the window closed before a selection count");
            }
        })
        .detach();
    }

    /// A selection count lands if its selection is still current.
    pub(crate) fn land_selection(&mut self, id: u64, held: &Arc<str>, words: u64, cx: &mut Context<Self>) {
        let slot = self.tab_mut(id).and_then(|tab| tab.extra.selection.as_mut());
        if let Some((_, counted)) = slot.filter(|(now, _)| Arc::ptr_eq(now, held)) {
            *counted = Some(words);
            cx.notify();
        }
    }

    /// The shown tab's count; a counted selection's words too.
    pub(crate) fn count_item(&self, cx: &App) -> Option<StatusBarItem> {
        let tab = self.active_tab()?;
        let counts = tab.extra.counts?;
        let selected = tab.extra.selection.as_ref().and_then(|(_, words)| *words);
        let label = ely_ui::i18n::word_count(cx, counts.words, counts.characters, selected);
        Some(StatusBarItem::new("mark-count").label(label))
    }
}

#[cfg(test)]
mod tests;
