//! Slide Show, Timer: a countdown for a slide.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    forms::{Choice, Input, Select, TextInput},
    overlays::Dialog as Card,
    primitives::Severity,
};
use ely_office::prelude::*;
use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, IntoElement as _, ParentElement as _, Styled as _,
    Window, div,
};

use super::{Clocks, Spot, Timer, length};
use crate::deck::Deck;
use crate::dialog::{Dialog, edit};
use crate::slideshow::presenter::clock;

/// The shown slide's countdown, as the dialog edits it.
pub struct Setting {
    pub tab: u64,
    /// The slide's name, and its number from one.
    slide: (String, usize),
    length: Entity<TextInput>,
    spot: Spot,
    /// Where it was dragged; a picked position forgets it.
    at: Option<(f32, f32)>,
    /// The slide has one already.
    held: bool,
}

/// The choice that keeps a dragged place.
const DRAGGED: &str = "dragged";

/// What a new countdown offers.
const OFFERED: &str = "5:00";

const SPOTS: [(Spot, &str, &str); 4] = [
    (Spot::Top, "top", "show.timer.top"),
    (Spot::Center, "center", "show.timer.center"),
    (Spot::Bottom, "bottom", "show.timer.bottom"),
    (Spot::Corner, "corner", "show.timer.corner"),
];

/// What the dialog's buttons ask.
#[derive(Clone, Copy)]
enum Asked {
    This,
    All,
    Remove,
}

impl Deck {
    pub fn ask_timer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.core.active() else {
            return;
        };
        let Some(extra) = self.tab(tab).map(|tab| &tab.extra) else {
            return;
        };
        // Masters never play.
        let current = extra.slides.current;
        let Some(slide) = extra.slides.list.get(current).filter(|_| !extra.masters) else {
            let text = Deck::text(cx, "show.score.master", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        let held = extra.timers.get(&slide.name).copied();
        let typed = held.map_or_else(|| OFFERED.to_string(), |timer| clock(timer.length.as_secs()));
        let slide = (slide.name.clone(), current + 1);
        let label = Deck::text(cx, "show.timer.length", &[]);
        let field = cx.new(|cx| TextInput::new(window, cx).label(label));
        field.update(cx, |field, cx| field.set_text(typed, cx));
        let spot = held.map_or_else(Spot::default, |timer| timer.spot);
        let at = held.and_then(|timer| timer.at);
        let setting = Setting { tab, slide, length: field, spot, at, held: held.is_some() };
        // Refused while another dialog is open; it says so.
        let _ = self.open_dialog(Dialog::Timer(setting), cx);
    }

    fn timer_asked(&mut self, asked: Asked, cx: &mut Context<Self>) {
        let Some(Dialog::Timer(setting)) = self.dialog.as_ref() else {
            return;
        };
        let (tab, name, place) = (setting.tab, setting.slide.0.clone(), (setting.spot, setting.at));
        let typed = setting.length.read(cx).text().to_string();
        let length = length(&typed);
        if length.is_none() && !matches!(asked, Asked::Remove) {
            // The dialog stays, to correct it.
            let text = Deck::text(cx, "show.timer.bad", &[]);
            return self.toast(text, Severity::Warning, cx);
        }
        self.dialog = None;
        self.set_timer(tab, (!matches!(asked, Asked::All)).then_some(name), length.filter(|_| !matches!(asked, Asked::Remove)), place, cx);
    }

    /// Gives slide `only`, or all sharing one clock, a countdown.
    pub fn set_timer(&mut self, tab: u64, only: Option<String>, length: Option<std::time::Duration>, (spot, at): (Spot, Option<(f32, f32)>), cx: &mut Context<Self>) {
        let held = self.tab(tab).and_then(|tab| tab.extra.clocks.clone());
        let clocks = held.unwrap_or_else(|| {
            let clocks = cx.new(|_| Clocks::default());
            cx.observe(&clocks, |_, _, cx| cx.notify()).detach();
            clocks
        });
        let id = clocks.update(cx, |clocks, _| clocks.fresh());
        let Some(extra) = self.tab_mut(tab).map(|tab| &mut tab.extra) else {
            return;
        };
        let names: Vec<String> = match only {
            Some(name) => vec![name],
            None => extra.slides.list.iter().map(|slide| slide.name.clone()).collect(),
        };
        for name in names {
            match length {
                Some(length) => extra.timers.insert(name, Timer { id, length, spot, at }),
                None => extra.timers.remove(&name),
            };
        }
        extra.clocks = Some(clocks);
        cx.notify();
    }

    /// Moves a slide's countdown; its clock runs on.
    pub fn drag_timer(&mut self, tab: u64, slide: &str, at: (f32, f32), cx: &mut Context<Self>) {
        let timer = self.tab_mut(tab).and_then(|tab| tab.extra.timers.get_mut(slide));
        if let Some(timer) = timer.filter(|timer| timer.at != Some(at)) {
            timer.at = Some(at);
            cx.notify();
        }
    }

    pub fn timer_dialog(&self, setting: &Setting, cx: &mut Context<Self>) -> AnyElement {
        let this = cx.entity();
        let label = |key: &str| Deck::text(cx, key, &[]);
        let mut choices: Vec<Choice> = SPOTS.iter().map(|(_, id, key)| Choice::new(*id, label(key))).collect();
        let mut picked = SPOTS.iter().find(|(spot, ..)| *spot == setting.spot).map_or(SPOTS[0].1, |found| found.1);
        if setting.at.is_some() {
            choices.push(Choice::new(DRAGGED, label("show.timer.dragged")));
            picked = DRAGGED;
        }
        let pick = this.clone();
        let select = Select::new("show-timer-spot", choices)
            .label(label("show.timer.place"))
            .selected(picked)
            .on_change(move |value, _, cx| {
                let found = SPOTS.iter().find(|(_, id, _)| *id == value.as_ref());
                edit(&pick, cx, |dialog, _| {
                    if let (Dialog::Timer(setting), Some((spot, ..))) = (dialog, found) {
                        (setting.spot, setting.at) = (*spot, None);
                    }
                })
            });
        let closing = this.clone();
        let dismiss = move |_: &mut Window, cx: &mut App| {
            closing.update(cx, |deck, cx| {
                deck.dialog = None;
                cx.notify();
            })
        };
        let title = Deck::text(cx, "show.timer.title", &[("number", &setting.slide.1.to_string())]);
        let apply = |id: &'static str, key: &str, asked: Asked, variant| {
            let (this, label) = (this.clone(), label(key));
            move |_| {
                Button::new(id, label)
                    .variant(variant)
                    .on_click(move |_, _, cx| this.update(cx, |deck, cx| deck.timer_asked(asked, cx)))
            }
        };
        let fields = DescriptionList::new()
            .item(label("show.timer.length"), div().w_full().max_w_64().child(Input::new(&setting.length)))
            .item(label("show.timer.place"), select);
        let card = Card::new("show-timer", title, dismiss)
            .detail(label("show.timer.detail"))
            .child(fields)
            .action(ely_office::cancel(cx));
        let card = match setting.held {
            true => card.action(apply("show-timer-remove", "show.timer.remove", Asked::Remove, ButtonVariant::Secondary)),
            false => card,
        };
        card.action(apply("show-timer-all", "show.score.all", Asked::All, ButtonVariant::Secondary))
            .action(apply("show-timer-this", "show.score.this", Asked::This, ButtonVariant::Primary))
            .into_any_element()
    }
}
