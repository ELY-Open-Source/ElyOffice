//! A document window: the first, or another beside it (W.1d).

use ely_office::{Core, prelude::*};
use ely_platform::AppId;
use ely_ui::Root;
use gpui::{App, AppContext as _, Context, Entity, Window};

use crate::docs::{Docs, menus};
use crate::files::Template;

/// Docs over `core` that keep points and saves.
pub(crate) fn docs(core: impl FnOnce(&mut Context<Docs>) -> Core<Docs>, cx: &mut App) -> Entity<Docs> {
    cx.new(|cx| {
        let mut host = Docs::new(core(cx), cx);
        // Recovery points, crash return, autosave (§6.2, §6.3).
        ely_office::Recovering::start_recovery(&mut host, cx);
        <Docs as ely_office::Autosaving>::start_autosave(cx);
        host
    })
}

/// The window's root, its home, and its close asking first.
pub(crate) fn root(docs: &Entity<Docs>, window: &mut Window, cx: &mut App) -> Entity<Root> {
    let root = ely_ui::document_root(AppId::Doc, docs.clone().into(), &ely_ui::commands::FILE, menus((false, false), cx), window, cx);
    docs.update(cx, |docs, cx| {
        docs.core.root = root.downgrade();
        docs.own_window(window, cx);
        window.focus(&docs.core.focus, cx);
    });
    let closing = docs.clone();
    ely_gpui_component::shell::guard_close(window, cx, move |window, cx| {
        closing.update(cx, |docs, cx| docs.should_close(window, cx))
    });
    root
}

/// Another window on the same engine; blank if `fresh`.
pub fn more(cx: &mut App, fresh: bool) -> std::io::Result<ely_ui::windows::Build> {
    let shared = ely_office::shared::Handle::<Docs>::joined(cx)?;
    Ok(Box::new(move |window, cx| {
        let docs = docs(move |cx| Core::join(shared, cx), cx);
        let root = root(&docs, window, cx);
        if fresh {
            docs.update(cx, |docs, cx| docs.new_from(Template::Blank, cx));
        }
        root
    }))
}

/// The docs in `window`, else in the front window.
pub(crate) fn front(window: Option<gpui::AnyWindowHandle>, cx: &App) -> Option<Entity<Docs>> {
    ely_office::shared::Handle::<Docs>::joined(cx).ok()?.front(window, cx)
}
