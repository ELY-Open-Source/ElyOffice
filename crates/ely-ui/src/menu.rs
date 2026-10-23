//! A right-click menu that opens once per ask.

use std::cell::Cell;

use ely_gpui_component::menus::{ContextMenu, Menu};
use gpui::{Pixels, Point};

/// One menu, asked for at a point in one tab.
pub struct Offer<W> {
    pub tab: u64,
    pub what: W,
    /// Bumps for each menu, so each opens once.
    request: (u64, Point<Pixels>),
    /// Handed to a host; a remounted host must not reopen.
    served: Cell<bool>,
}

impl<W> Offer<W> {
    /// The next menu after `last`.
    pub fn new(last: Option<&Offer<W>>, tab: u64, what: W, at: Point<Pixels>) -> Self {
        let number = last.map_or(1, |last| last.request.0 + 1);
        Offer { tab, what, request: (number, at), served: Cell::new(false) }
    }

    /// Its host; only the first render opens it.
    pub fn host(&self, id: &'static str, menu: Menu) -> ContextMenu {
        let request = (!self.served.replace(true)).then_some(self.request);
        ContextMenu::new(id, menu).manual(request)
    }
}
