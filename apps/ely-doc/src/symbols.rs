//! Insert Symbol: common characters typed at the cursor.

use ely_gpui_component::{buttons::Button, overlays::Dialog as Card, theme::ControlSize};
use gpui::{
    App, Context, Entity, ParentElement as _, SharedString, Styled as _,
    Window, div,
};

use crate::actions::Dialog;
use crate::docs::Docs;
use ely_office::prelude::*;

gpui::actions!(doc, [InsertSymbol]);

/// Punctuation, currency, math, arrows, Greek, fractions, marks.
const SYMBOLS: &str = "©®™§¶†‡•…‰′″°±×÷≠≈≤≥∞√∑∏∫∂∆∇∈∉∩∪⊂⊃⊆⊇∧∨¬∀∃∴←↑→↓↔⇐⇒⇔€£¥¢₹₩₽₿«»‹›“”‘’„‚–—¡¿½⅓⅔¼¾⅛µαβγδεζηθικλμνξοπρστυφχψωΑΒΓΔΘΛΞΠΣΦΨΩ✓✗★☆♠♣♥♦♪☺";

impl Docs {
    pub fn on_symbol(&mut self, _: &InsertSymbol, _: &mut Window, cx: &mut Context<Self>) {
        if self.active_tab().is_none() {
            return self.no_document(cx);
        }
        self.dialog = Some(Dialog::Symbols);
        cx.notify();
    }

    /// Types `symbol` into the active document and closes the dialog.
    pub(crate) fn type_symbol(&mut self, symbol: char, cx: &mut Context<Self>) {
        self.dialog = None;
        cx.notify();
        // The tab may close while the grid is open.
        let Some(tab) = self.active_tab() else {
            return self.no_document(cx);
        };
        tab.view.update(cx, |view, _| view.insert(&symbol.to_string()));
    }
}

/// The Insert Symbol dialog.
pub fn symbol_dialog(
    this: Entity<Docs>,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let grid = SYMBOLS.chars().enumerate().fold(
        div().flex().flex_wrap().gap_1().max_w_96(),
        |grid, (index, symbol)| {
            let this = this.clone();
            grid.child(
                Button::new(
                    SharedString::from(format!("doc-symbol-{index}")),
                    symbol.to_string(),
                )
                .size(ControlSize::Sm)
                .on_click(move |_, _, cx| this.update(cx, |docs, cx| docs.type_symbol(symbol, cx))),
            )
        },
    );
    let cancel = Docs::text(cx, "office.cancel", &[]);
    Card::new("doc-symbols", Docs::text(cx, "doc.symbol.title", &[]), dismiss)
        .child(grid)
        .action(move |close| {
            Button::new("doc-symbols-cancel", cancel)
                .on_click(move |_, window, cx| close(window, cx))
        })
}
