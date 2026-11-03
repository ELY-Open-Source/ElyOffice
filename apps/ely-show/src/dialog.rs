//! ElyShow's one open dialog.

use ely_gpui_component::{forms::TextInput, primitives::Severity};
use ely_office::prelude::*;
use gpui::{App, Context, Entity, IntoElement as _};

use crate::deck::Deck;

/// What the open dialog asks.
pub enum Dialog {
    New(crate::new::Choosing),
    Table {
        tab: u64,
        columns: u16,
        rows: u16,
    },
    Chart(Box<crate::chart::Editing>),
    /// A new comment, or an edit of one.
    Remark {
        tab: u64,
        edit: Option<String>,
        field: Entity<TextInput>,
    },
    Show(crate::slideshow::start::Starting),
    Output(crate::output::Output),
    Check(Vec<crate::check::Finding>),
}

/// Changes the open dialog's state.
pub(crate) fn edit(
    this: &Entity<Deck>,
    cx: &mut App,
    change: impl FnOnce(&mut Dialog, &mut Context<Deck>),
) {
    this.update(cx, |deck, cx| {
        if let Some(mut dialog) = deck.dialog.take() {
            change(&mut dialog, cx);
            deck.dialog = Some(dialog);
        }
        cx.notify();
    });
}

impl Deck {
    /// Opens `dialog`, or hands it back while another is open.
    pub(crate) fn open_dialog(&mut self, dialog: Dialog, cx: &mut Context<Self>) -> Result<(), Dialog> {
        if self.dialog.is_some() {
            let text = Deck::text(cx, "show.dialog.busy", &[]);
            self.toast(text, Severity::Warning, cx);
            return Err(dialog);
        }
        self.dialog = Some(dialog);
        cx.notify();
        Ok(())
    }

    /// A read that will open a dialog; earlier ones lapse.
    pub fn ask_dialog(&mut self) -> u64 {
        self.dialog_asked += 1;
        self.dialog_asked
    }

    /// A dialog or tab change ends earlier asks.
    pub(crate) fn watch_dialogs(&mut self) {
        let now = (self.dialog.as_ref().map(std::mem::discriminant), self.core.active());
        if self.dialog_seen != now {
            self.dialog_seen = now;
            self.dialog_asked += 1;
        }
    }

    /// Whether read `asked` still stands; `open_dialog` checks the rest.
    pub fn dialog_due(&self, asked: u64, id: u64) -> bool {
        self.dialog_asked == asked && self.core.active() == Some(id)
    }

    /// The open dialog, drawn.
    pub fn dialog_element(&mut self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        if matches!(self.dialog, Some(Dialog::New(_))) {
            return self.new_dialog(cx);
        }
        Some(match self.dialog.as_ref()? {
            Dialog::New(_) => unreachable!("drawn above"),
            Dialog::Table { columns, rows, .. } => self.table_dialog((*columns, *rows), cx),
            Dialog::Chart(editing) => self.chart_dialog(editing, cx),
            Dialog::Remark { tab, edit, field } => self.remark_dialog((*tab, edit), field, cx),
            Dialog::Show(starting) => self.show_dialog(starting, cx),
            Dialog::Check(found) => self.check_dialog(found, cx),
            Dialog::Output(output) => {
                let closing = cx.entity();
                let dismiss = move |_: &mut gpui::Window, cx: &mut App| {
                    closing.update(cx, |deck, cx| {
                        deck.dialog = None;
                        cx.notify();
                    })
                };
                crate::output::output_dialog(cx.entity(), output, dismiss, cx).into_any_element()
            }
        })
    }
}
