//! A workbook window: the first, or another beside it (W.1b).

use ely_office::{Core, prelude::*};
use ely_platform::AppId;
use ely_ui::Root;
use gpui::{App, AppContext as _, Context, Entity, Window};

use crate::book::{Book, menus};

/// A book over `core` that keeps points and saves.
pub(crate) fn book(core: impl FnOnce(&mut Context<Book>) -> Core<Book>, window: &mut Window, cx: &mut App) -> Entity<Book> {
    cx.new(|cx| {
        let mut host = Book::new(core(cx), window, cx);
        // Recovery points, crash return, autosave (§6.2, §6.3).
        ely_office::Recovering::start_recovery(&mut host, cx);
        <Book as ely_office::Autosaving>::start_autosave(cx);
        host
    })
}

/// The window's root, its home, and its close asking first.
pub(crate) fn root(book: &Entity<Book>, window: &mut Window, cx: &mut App) -> Entity<Root> {
    let root = ely_ui::document_root(AppId::Table, book.clone().into(), &ely_ui::commands::FILE, menus((false, false), cx), window, cx);
    book.update(cx, |book, cx| {
        book.core.root = root.downgrade();
        book.own_window(window, cx);
        window.focus(&book.core.focus, cx);
    });
    let closing = book.clone();
    ely_gpui_component::shell::guard_close(window, cx, move |window, cx| {
        closing.update(cx, |book, cx| book.should_close(window, cx))
    });
    root
}

/// Another window on the same engine, with a blank book.
pub fn more(cx: &mut App) -> std::io::Result<ely_ui::windows::Build> {
    let shared = ely_office::shared::Handle::<Book>::joined(cx)?;
    Ok(Box::new(move |window, cx| {
        let book = book(move |cx| Core::join(shared, cx), window, cx);
        let root = root(&book, window, cx);
        book.update(cx, |book, cx| book.new_blank(cx));
        root
    }))
}

/// The book in `window`, else in the front window.
pub(crate) fn front(window: Option<gpui::AnyWindowHandle>, cx: &App) -> Option<Entity<Book>> {
    ely_office::shared::Handle::<Book>::joined(cx).ok()?.front(window, cx)
}
