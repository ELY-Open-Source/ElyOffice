//! Keys typed into a tab still loading (R15.57).

use std::ops::Range;

use gpui::{
    AnyElement, Bounds, Context, ElementInputHandler, Entity, EntityInputHandler, FocusHandle, InteractiveElement as _,
    IntoElement as _, KeyDownEvent, Keystroke, ParentElement as _, Pixels, Styled as _, UTF16Selection, Window, canvas,
    div,
};

use crate::{Engine as _, Host, Tabs as _};

/// Input for loading tabs, kept until each is shown.
pub struct Hold {
    focus: FocusHandle,
    /// The loading tab now taking the keys.
    tab: Option<u64>,
    /// Each tab's keys and text, as typed.
    held: Vec<(u64, Keystroke)>,
    /// The IME's text not yet committed.
    marked: Option<String>,
}

impl Hold {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        Hold { focus: cx.focus_handle(), tab: None, held: Vec::new(), marked: None }
    }

    /// Keys now go to loading tab `tab`.
    pub(crate) fn take_keys(&mut self, tab: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.commit();
        self.tab = Some(tab);
        window.focus(&self.focus, cx);
    }

    /// Tab `tab`'s input, in order, as keystrokes.
    fn take(&mut self, tab: u64) -> Vec<Keystroke> {
        if self.tab == Some(tab) {
            self.commit();
        }
        let (taken, rest) = std::mem::take(&mut self.held).into_iter().partition(|(of, _)| *of == tab);
        self.held = rest;
        taken.into_iter().map(|(_, key)| key).collect()
    }

    /// Tab `tab` has its view; the hold lets go.
    fn release(&mut self, tab: u64) {
        if self.tab == Some(tab) {
            self.tab = None;
        }
    }

    fn keep(&mut self, key: Keystroke) {
        if let Some(tab) = self.tab {
            self.held.push((tab, key));
        }
    }

    /// Text the IME gave; it reaches the view's input handler.
    fn text(&mut self, text: &str) {
        let key = Keystroke { modifiers: Default::default(), key: String::new(), key_char: Some(text.into()) };
        self.keep(key);
    }

    fn commit(&mut self) {
        if let Some(marked) = self.marked.take() {
            self.text(&marked);
        }
    }

    fn key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        let m = key.modifiers;
        // Text comes through the IME, as gpui sends it.
        let typed = key.key_char.is_some()
            && !matches!(key.key.as_str(), "enter" | "tab")
            && !(m.control || m.platform || m.alt || m.function);
        if self.tab.is_none() || typed {
            return;
        }
        self.keep(key.clone());
        cx.stop_propagation();
    }
}

impl EntityInputHandler for Hold {
    fn text_for_range(&mut self, _: Range<usize>, _: &mut Option<Range<usize>>, _: &mut Window, _: &mut Context<Self>) -> Option<String> {
        None
    }

    fn selected_text_range(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) -> Option<UTF16Selection> {
        Some(UTF16Selection { range: 0..0, reversed: false })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked.as_ref().map(|marked| 0..marked.encode_utf16().count())
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.commit();
    }

    fn replace_text_in_range(&mut self, _: Option<Range<usize>>, text: &str, _: &mut Window, _: &mut Context<Self>) {
        self.marked = None;
        self.text(text);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        text: &str,
        _: Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.marked = (!text.is_empty()).then(|| text.to_string());
    }

    fn bounds_for_range(&mut self, _: Range<usize>, _: Bounds<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<Bounds<Pixels>> {
        None
    }

    fn character_index_for_point(&mut self, _: gpui::Point<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        None
    }
}

/// `body` takes the keys meant for a loading tab.
pub(crate) fn around(hold: &Entity<Hold>, body: AnyElement, cx: &gpui::App) -> AnyElement {
    let focus = hold.read(cx).focus.clone();
    let (bound, keys, input) = (hold.clone(), hold.clone(), hold.clone());
    let keep = move |key, cx: &mut gpui::App| bound.update(cx, |hold, _| hold.keep(key));
    ely_lok::bound_keys(div(), keep)
        .size_full()
        .relative()
        .track_focus(&focus)
        .on_key_down(move |event, _, cx| keys.update(cx, |hold, cx| hold.key_down(event, cx)))
        .child(body)
        .child(
            canvas(|_, _, _| (), move |bounds, (), window, cx| window.handle_input(&focus, ElementInputHandler::new(bounds, input), cx))
                .absolute()
                .size_full(),
        )
        .into_any_element()
}

/// Closed tab `id`'s typed input goes with it.
pub(crate) fn forget<H: Host>(host: &H, id: u64, cx: &mut Context<H>) {
    let lost = host.core().hold.update(cx, |hold, _| {
        hold.release(id);
        hold.take(id).len()
    });
    if lost > 0 {
        log::info!("{}: tab {id} closed with {lost} inputs typed while it loaded", H::KIND.app.slug());
    }
}

/// Whether the keys wait for loading tab `id`.
pub(crate) fn holds<H: Host>(host: &H, id: u64, window: &Window, cx: &gpui::App) -> bool {
    let hold = host.core().hold.read(cx);
    hold.tab == Some(id) && hold.focus.is_focused(window)
}

/// Shown tab `id` gets what was typed while it loaded.
pub(crate) fn replay<H: Host>(host: &mut H, id: u64, window: &mut Window, cx: &mut Context<H>) {
    let Some(view) = host.tab(id).map(|tab| tab.view.clone()) else {
        return;
    };
    let keys = host.core().hold.update(cx, |hold, _| hold.take(id));
    if keys.is_empty() {
        return;
    }
    log::info!("{}: tab {id} takes {} inputs typed while it loaded", H::KIND.app.slug(), keys.len());
    H::Engine::replay(&view, keys, window, cx);
}

/// Landed tab `id` takes the keys before its first frame.
pub(crate) fn hand_over<H: Host>(id: u64, window: &mut Window, cx: &mut Context<H>) {
    // Until then the hold keeps catching them.
    let host = cx.weak_entity();
    window.on_next_frame(move |window, cx| {
        let handed = host.update(cx, |host, cx| {
            replay(host, id, window, cx);
            host.core().hold.update(cx, |hold, _| hold.release(id));
            if host.core().active() == Some(id)
                && let Some(tab) = host.tab(id)
            {
                window.focus(&H::Engine::focus(tab.view.read(cx)), cx);
            }
        });
        if handed.is_err() {
            log::info!("office: tab {id} landed after its window");
        }
    });
}
