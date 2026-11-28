//! The screen shows play on: picked once, remembered.

use std::rc::Rc;

use ely_gpui_component::{
    buttons::ButtonVariant,
    menus::{DropdownMenu, Menu, MenuItem},
};
use ely_platform::{AppId, settings::Store};
use gpui::{App, AppContext as _, BorrowAppContext as _, Context, Global, IntoElement, PlatformDisplay, SharedString, Window};

use crate::deck::Deck;

const KEY: &str = "show_screen";

/// The remembered screen's id; None until picked.
struct Remembered(Option<String>);

impl Global for Remembered {}

fn id_of(display: &Rc<dyn PlatformDisplay>) -> Option<String> {
    match display.uuid() {
        Ok(id) => Some(id.to_string()),
        Err(error) => {
            log::error!("show: a screen has no id: {error}");
            None
        }
    }
}

fn stored() -> Option<String> {
    match Store::app(AppId::Show).and_then(|store| store.read()) {
        Ok(fields) => fields.get(KEY).and_then(|value| value.as_str()).map(Into::into),
        Err(error) => {
            log::error!("show: the chosen screen is unreadable: {error}");
            None
        }
    }
}

fn remembered(cx: &mut App) -> Option<String> {
    if !cx.has_global::<Remembered>() {
        cx.set_global(Remembered(stored()));
    }
    cx.global::<Remembered>().0.clone()
}

/// The screens, numbered left to right, then down.
pub fn list(cx: &App) -> Vec<Rc<dyn PlatformDisplay>> {
    let mut screens = cx.displays();
    screens.sort_by(|a, b| {
        let (a, b) = (a.bounds().origin, b.bounds().origin);
        f32::from(a.x).total_cmp(&f32::from(b.x)).then(f32::from(a.y).total_cmp(&f32::from(b.y)))
    });
    screens
}

/// The remembered screen if there, else a secondary one.
fn pick_among(ids: &[Option<String>], remembered: Option<&str>, secondary: impl Fn(usize) -> bool) -> usize {
    let kept = remembered.and_then(|kept| ids.iter().position(|id| id.as_deref() == Some(kept)));
    kept.or_else(|| (0..ids.len()).find(|at| secondary(*at))).unwrap_or(0)
}

/// The chosen screen's place among the screens.
pub fn chosen(cx: &mut App) -> usize {
    let kept = remembered(cx);
    let screens = list(cx);
    let primary = cx.primary_display().map(|display| display.id());
    let ids: Vec<_> = screens.iter().map(id_of).collect();
    pick_among(&ids, kept.as_deref(), |at| primary.is_some_and(|primary| screens[at].id() != primary))
}

/// Keeps screen `at` for this and later shows.
pub fn remember(at: usize, cx: &mut App) {
    let Some(id) = list(cx).get(at).and_then(id_of) else {
        return;
    };
    remembered(cx);
    cx.update_global(|kept: &mut Remembered, _| kept.0 = Some(id.clone()));
    cx.background_spawn(async move {
        if let Err(error) = Store::app(AppId::Show).and_then(|store| store.set(KEY, id.into())) {
            log::error!("show: the chosen screen was not saved: {error}");
        }
    })
    .detach();
}

/// A screen's number and size, as listed.
pub fn name(at: usize, display: &Rc<dyn PlatformDisplay>, cx: &App) -> SharedString {
    let size = display.bounds().size;
    Deck::text(
        cx,
        "show.play.screen",
        &[
            ("number", &(at + 1).to_string()),
            ("width", &f32::from(size.width).round().to_string()),
            ("height", &f32::from(size.height).round().to_string()),
        ],
    )
}

/// The status bar's screen list; None with one screen.
pub fn picker(cx: &mut Context<Deck>) -> Option<impl IntoElement + use<>> {
    let screens = list(cx);
    if screens.len() < 2 {
        return None;
    }
    let current = chosen(cx);
    let deck = cx.entity();
    let menu = screens.iter().enumerate().fold(Menu::new(), |menu, (at, display)| {
        let deck = deck.clone();
        let item = MenuItem::radio(name(at, display, cx), at == current);
        menu.item(item.on_click(move |window, cx| deck.update(cx, |deck, cx| deck.choose_screen(at, window, cx))))
    });
    let label = name(current, &screens[current], cx);
    Some(DropdownMenu::new("show-screen", label, menu).variant(ButtonVariant::Ghost))
}

impl Deck {
    /// Remembers screen `at` and takes this window there.
    pub fn choose_screen(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(display) = list(cx).get(at).map(|display| display.id()) else {
            return;
        };
        remember(at, cx);
        ely_ui::move_to_display(window, display, cx).detach();
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::pick_among;

    #[test]
    fn the_remembered_screen_wins_and_a_secondary_one_starts() {
        let ids = [Some("a".to_string()), Some("b".to_string()), Some("c".to_string())];
        let secondary = |at| at != 1;
        assert_eq!(pick_among(&ids, Some("b"), secondary), 1, "remembered, even if primary");
        assert_eq!(pick_among(&ids, None, |at| at != 0), 1, "first time: a secondary one");
        assert_eq!(pick_among(&ids, Some("gone"), secondary), 0, "unplugged: a secondary one");
        assert_eq!(pick_among(&ids[..1], Some("gone"), |_| false), 0, "one screen");
    }
}
