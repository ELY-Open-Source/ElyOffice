//! Ely's right-click menus, gated by the kernel's (DOC-02).

use std::io;

use ely_gpui_component::{
    menus::{Menu, MenuItem},
    primitives::IconName,
};
use ely_lok::{LokView, menu::Offered};
use gpui::{App, Entity, Pixels, Point};

/// The view's last kernel menu and where it opens.
pub fn take(view: &Entity<LokView>, cx: &mut App) -> Option<io::Result<(Offered, Point<Pixels>)>> {
    let (json, at) = view.update(cx, |view, _| view.take_menu())?;
    let offered = match ely_lok::menu::read(&json) {
        Ok(offered) => offered,
        Err(error) => return Some(Err(error)),
    };
    // Without a click it opens at the caret.
    let caret = || view.read(cx).cursor_bounds().map(|caret| caret.bottom_left());
    let Some(at) = at.or_else(caret) else {
        log::warn!("office: a menu with no click and no caret");
        return None;
    };
    log::info!("office: a {:?} menu", offered.target);
    Some(Ok((offered, at)))
}

/// A menu row: its label key and its gating command.
pub trait Pick: Copy + 'static {
    fn row(self) -> (&'static str, &'static str);
}

/// A row, a submenu or a line.
pub enum Entry<P: 'static> {
    Row(P),
    Sub(&'static str, &'static [P]),
    Line,
}

/// Cut, copy and paste as one row of icons.
pub fn clipboard<P: Pick>(menu: Menu, picks: [P; 3], item: impl Fn(P) -> MenuItem) -> Menu {
    let icons = [IconName::Scissors, IconName::Copy, IconName::Clipboard];
    let row = picks.into_iter().zip(icons).map(|(pick, icon)| item(pick).icon(icon));
    menu.buttons(row)
}

/// Adds entries; a submenu is on when any row is.
pub fn entries<P: Pick>(
    menu: Menu,
    entries: &[Entry<P>],
    cx: &App,
    on: impl Fn(P) -> bool,
    item: impl Fn(P) -> MenuItem,
) -> Menu {
    entries.iter().fold(menu, |menu, entry| match entry {
        Entry::Row(pick) => menu.item(item(*pick)),
        Entry::Sub(key, picks) => {
            let sub = picks.iter().fold(Menu::new(), |sub, pick| sub.item(item(*pick)));
            let any = picks.iter().any(|pick| on(*pick));
            menu.item(MenuItem::submenu(crate::text(cx, key, &[]), sub).disabled(!any))
        }
        Entry::Line => menu.separator(),
    })
}
