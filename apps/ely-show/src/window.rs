//! A deck window: the first, or another beside it (W.1d).

use ely_office::{Core, prelude::*};
use ely_platform::AppId;
use ely_ui::Root;
use gpui::{App, AppContext as _, Context, Entity, Window};

use crate::deck::Deck;

/// A deck over `core` that keeps points and saves.
pub(crate) fn deck(core: impl FnOnce(&mut Context<Deck>) -> Core<Deck>, cx: &mut App) -> Entity<Deck> {
    cx.new(|cx| {
        let mut host = Deck::new(core(cx), cx);
        // Recovery points, crash return, autosave (§6.2, §6.3).
        ely_office::Recovering::start_recovery(&mut host, cx);
        <Deck as ely_office::Autosaving>::start_autosave(cx);
        host
    })
}

/// The window's root, its home, and its close asking first.
pub(crate) fn root(deck: &Entity<Deck>, window: &mut Window, cx: &mut App) -> Entity<Root> {
    let menus = menus((false, false, false), (false, false), cx);
    let root = ely_ui::document_root(AppId::Show, deck.clone().into(), &ely_ui::commands::FILE, menus, window, cx);
    deck.update(cx, |deck, cx| {
        deck.core.root = root.downgrade();
        deck.own_window(window, cx);
        window.focus(&deck.core.focus, cx);
    });
    let closing = deck.clone();
    ely_gpui_component::shell::guard_close(window, cx, move |window, cx| {
        closing.update(cx, |deck, cx| deck.should_close(window, cx))
    });
    root
}

/// Another window on the same engine; blank if `fresh`.
pub fn more(cx: &mut App, fresh: bool) -> std::io::Result<ely_ui::windows::Build> {
    let shared = ely_office::shared::Handle::<Deck>::joined(cx)?;
    Ok(Box::new(move |window, cx| {
        let deck = deck(move |cx| Core::join(shared, cx), cx);
        let root = root(&deck, window, cx);
        if fresh {
            deck.update(cx, |deck, cx| deck.new_blank(cx));
        }
        root
    }))
}

/// The deck in `window`, else in the front window.
pub(crate) fn front(window: Option<gpui::AnyWindowHandle>, cx: &App) -> Option<Entity<Deck>> {
    ely_office::shared::Handle::<Deck>::joined(cx).ok()?.front(window, cx)
}

/// ElyShow's menus after File.
pub fn menus(on: (bool, bool, bool), undo: (bool, bool), cx: &App) -> Vec<gpui::Menu> {
    let [insert, view] = crate::insert::menus(on, cx);
    let edit = gpui::Menu::new(ely_ui::i18n::text(cx, "menu.edit", &[])).items(ely_office::undo_items(undo, cx));
    vec![edit, crate::slides::menu(cx), insert, view, crate::slideshow::menu(cx)]
}
