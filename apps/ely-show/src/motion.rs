//! The shown slide's transition and effects (SHOW-09).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    forms::{Choice, NumberInput, Select, Switch},
    lists::{ListItem, SelectableList},
    menus::{DropdownMenu, Menu, MenuItem},
    theme::ControlSize,
};
use ely_lok::show::{EFFECTS, Effect, TRANSITIONS, Transition};
use ely_office::prelude::*;
use gpui::{
    AnyElement, App, Context, IntoElement, ParentElement as _, SharedString, Styled as _, div,
};
use serde_json::json;

use crate::deck::{Deck, Extra};
use crate::design::{Panel, on_deck};

/// Ely's name for an effect, else the kernel's.
fn effect_name(effect: &Effect, cx: &App) -> SharedString {
    match EFFECTS.contains(&effect.preset.as_str()) {
        true => Deck::text(cx, &format!("show.effect.{}", &effect.preset[4..]), &[]),
        false => effect.label.clone().into(),
    }
}

/// Seconds sent once, though Enter and blur both commit.
pub(crate) fn dedupe(
    panel: gpui::Entity<Panel>,
    field: impl Into<String>,
    now: f64,
    send: impl Fn(&mut Deck, f64, &mut Context<Deck>) + Clone + 'static,
) -> impl Fn(f64, &mut gpui::Window, &mut App) + 'static {
    let ms = |seconds: f64| (seconds * 1000.0).round() as i64;
    let field = field.into();
    move |n, _, cx| {
        let send = send.clone();
        let deliver = move |deck: &mut Deck, cx: &mut Context<Deck>| send(deck, n, cx);
        crate::object::commit(&panel, field.clone(), (ms(n), Some(ms(now))), deliver, cx)
    }
}

/// A field of one effect on one slide.
pub(crate) fn effect_key(field: &str, slide: &str, (at, effect): (usize, &Effect)) -> String {
    format!("effect.{field}.{slide}.{at}.{}", effect.hash)
}

fn number(id: &'static str, label: SharedString, now: f64, range: (f64, f64)) -> NumberInput {
    NumberInput::new(id, now)
        .label(label)
        .range(range.0, range.1)
        .step(0.1)
        .precision(2)
        .size(ControlSize::Sm)
}

impl Extra {
    fn slide_name(&self) -> Option<&String> {
        Some(&self.slides.list.get(self.slides.current)?.name)
    }

    /// The transition to show: sent, else read.
    pub(crate) fn shown_transition(&self) -> Option<Transition> {
        match (&self.transition_asked, self.slide_name()) {
            (Some((slide, asked, _)), Some(now)) if slide == now => Some(asked.clone()),
            _ => Some(self.motion.as_ref()?.0.clone()),
        }
    }

    /// The input a read may settle: landed before it began.
    pub(crate) fn transition_landed(&self, view: &ely_lok::LokView) -> Option<u64> {
        let (_, _, seq) = self.transition_asked.as_ref()?;
        view.reached(*seq).then_some(*seq)
    }

    /// Forgets the sent transition once read after it or left.
    pub(crate) fn settle_transition(&mut self, landed: Option<u64>) {
        let gone = match (&self.transition_asked, self.slide_name()) {
            (Some((slide, _, seq)), Some(now)) => slide != now || landed == Some(*seq),
            (asked, _) => asked.is_some(),
        };
        if gone {
            self.transition_asked = None;
        }
    }
}

impl Deck {
    /// Edits the transition, atop any unread send.
    pub(crate) fn send_transition(
        &mut self,
        edit: impl FnOnce(&mut Transition),
        all: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.core.active().and_then(|id| self.tab_mut(id)) else {
            return;
        };
        let (Some(mut next), Some(slide)) = (
            tab.extra.shown_transition(),
            tab.extra.slide_name().cloned(),
        ) else {
            return;
        };
        edit(&mut next);
        let (id, view) = (tab.id, tab.view.clone());
        let json = ely_lok::show::transition_json(&next, all);
        self.run_in(Some(id), "slide.transition", &[&json], cx);
        let seq = view.read(cx).sent();
        if let Some(tab) = self.tab_mut(id) {
            tab.extra.transition_asked = Some((slide, next, seq));
        }
        self.force_reread(id, cx);
    }

    pub(crate) fn send_effect(
        &mut self,
        index: usize,
        change: serde_json::Value,
        cx: &mut Context<Self>,
    ) {
        let mut json = change;
        json["index"] = json!(index);
        self.design_command("animation.change", &[&json.to_string()], cx);
    }
}

impl Panel {
    /// The slide's transition and effects; none on a master.
    pub(crate) fn motion_view(&self, picked: bool, cx: &mut Context<Self>) -> Option<AnyElement> {
        let deck = self.deck.upgrade()?;
        let extra = &deck.read(cx).active_tab()?.extra;
        let (transition, effects) = (extra.shown_transition()?, extra.motion.clone()?.1);
        let slide = extra.slide_name()?.clone();
        let rows = self.transition_rows(&transition, &slide, cx);
        let effects = self.effect_rows(&effects, (picked, &slide), cx);
        Some(
            div()
                .px_3()
                .flex()
                .flex_col()
                .gap_3()
                .child(rows)
                .child(effects)
                .into_any_element(),
        )
    }

    fn transition_rows(
        &self,
        now: &Transition,
        slide: &str,
        cx: &mut Context<Self>,
    ) -> DescriptionList {
        let label = |key: &str| Deck::text(cx, key, &[]);
        let weak = self.deck.clone();
        let change = move |cx: &mut App, edit: &dyn Fn(&mut Transition)| {
            on_deck(&weak, cx, |deck, cx| deck.send_transition(edit, false, cx));
        };
        let mut choices: Vec<Choice> = (TRANSITIONS.iter())
            .map(|id| {
                Choice::new(
                    *id,
                    label(&format!(
                        "show.transition.{}",
                        if id.is_empty() { "none" } else { id }
                    )),
                )
            })
            .collect();
        if !TRANSITIONS.contains(&now.preset.as_str()) {
            choices.push(Choice::new(
                now.preset.clone(),
                label("show.transition.other"),
            ));
        }
        let pick = change.clone();
        let kind = Select::new("show-transition", choices)
            .label(label("show.motion.transition"))
            .selected(now.preset.clone())
            .on_change(move |value, _, cx| {
                let value = value.to_string();
                pick(cx, &|t| t.preset = value.clone())
            });
        let duration = number(
            "show-transition-duration",
            label("show.motion.duration"),
            now.duration,
            (0.1, 60.0),
        )
        .on_commit(dedupe(
            cx.entity(),
            format!("transition.duration.{slide}"),
            now.duration,
            |deck, n, cx| deck.send_transition(|t| t.duration = n, false, cx),
        ));
        let flip = change.clone();
        let auto = Switch::new("show-transition-auto", now.auto)
            .name(label("show.motion.auto"))
            .on_change(move |on, _, cx| flip(cx, &|t| t.auto = on));
        let mut rows = DescriptionList::new()
            .item(label("show.motion.transition"), kind)
            .item(label("show.motion.duration"), div().w_full().max_w_32().child(duration))
            .item(label("show.motion.auto"), auto);
        if now.auto {
            let time = number(
                "show-transition-time",
                label("show.motion.time"),
                now.time,
                (0.0, 3600.0),
            )
            .on_commit(dedupe(
                cx.entity(),
                format!("transition.time.{slide}"),
                now.time,
                |deck, n, cx| deck.send_transition(|t| t.time = n, false, cx),
            ));
            rows = rows.item(label("show.motion.time"), div().w_full().max_w_32().child(time));
        }
        let (everyone, preview) = (self.deck.clone(), self.deck.clone());
        let buttons = div()
            .flex()
            .flex_wrap()
            .gap_1()
            .child(
                Button::new("show-transition-all", label("show.motion.all"))
                    .size(ControlSize::Sm)
                    .on_click(move |_, _, cx| {
                        on_deck(&everyone, cx, |deck, cx| {
                            deck.send_transition(|_| {}, true, cx)
                        })
                    }),
            )
            .child(
                Button::new(
                    "show-transition-preview",
                    label("show.motion.preview_transition"),
                )
                .size(ControlSize::Sm)
                .on_click(move |_, _, cx| {
                    on_deck(&preview, cx, |deck, cx| deck.preview_transition(cx))
                }),
            );
        rows.item(SharedString::default(), buttons)
    }

    fn effect_rows(
        &self,
        effects: &[Effect],
        (picked, slide): (bool, &str),
        cx: &mut Context<Self>,
    ) -> DescriptionList {
        let label = |key: &str| Deck::text(cx, key, &[]);
        let current = self.effect.filter(|at| *at < effects.len());
        let mut list = SelectableList::new("show-effects").label(label("show.motion.effects"));
        for (at, effect) in effects.iter().enumerate() {
            let start = label(&format!("show.effect.start.{}", effect.start));
            let item = ListItem::new(
                SharedString::from(format!("show-effect-{at}")),
                effect_name(effect, cx),
            )
            .description(start);
            list = list.row(SharedString::from(at.to_string()), item);
        }
        let this = cx.entity();
        let list = list
            .selected(
                current
                    .map(|at| SharedString::from(at.to_string()))
                    .into_iter()
                    .collect::<Vec<_>>(),
            )
            .on_change(move |keys, _, cx| {
                let at = keys.first().and_then(|key| key.parse().ok());
                this.update(cx, |panel, cx| {
                    panel.effect = at;
                    cx.notify();
                })
            });
        let mut menu = Menu::new();
        for (n, preset) in EFFECTS.iter().enumerate() {
            if n > 0 && EFFECTS[n - 1].split('-').nth(1) != preset.split('-').nth(1) {
                menu = menu.separator();
            }
            let weak = self.deck.clone();
            let name = label(&format!("show.effect.{}", &preset[4..]));
            menu = menu.item(
                MenuItem::new(name)
                    .disabled(!picked)
                    .on_click(move |_, cx| {
                        let json = json!({ "preset": preset }).to_string();
                        on_deck(&weak, cx, |deck, cx| {
                            deck.design_command("animation.add", &[&json], cx)
                        })
                    }),
            );
        }
        let preview = self.deck.clone();
        let head = div()
            .flex()
            .flex_wrap()
            .gap_1()
            .child(DropdownMenu::new(
                "show-effect-add",
                label("show.motion.add"),
                menu,
            ))
            .child(
                Button::new("show-effect-preview", label("show.motion.preview_effects"))
                    .size(ControlSize::Sm)
                    .disabled(effects.is_empty())
                    .on_click(move |_, _, cx| {
                        on_deck(&preview, cx, |deck, cx| deck.preview_effects(cx))
                    }),
            );
        let mut rows = DescriptionList::new().stacked().item(
            label("show.motion.effects"),
            div().flex().flex_col().gap_1().child(head).child(list),
        );
        if let Some(at) = current {
            rows = self.effect_controls(rows, (at, &effects[at], slide), effects.len(), cx);
        }
        rows
    }

    fn effect_controls(
        &self,
        rows: DescriptionList,
        (at, effect, slide): (usize, &Effect, &str),
        count: usize,
        cx: &mut Context<Self>,
    ) -> DescriptionList {
        let label = |key: &str| Deck::text(cx, key, &[]);
        let send = |change: serde_json::Value| {
            let weak = self.deck.clone();
            move |cx: &mut App| {
                let change = change.clone();
                on_deck(&weak, cx, move |deck, cx| deck.send_effect(at, change, cx))
            }
        };
        let starts = ["click", "with", "after"]
            .map(|key| Choice::new(key, label(&format!("show.effect.start.{key}"))));
        let weak = self.deck.clone();
        let start = Select::new("show-effect-start", starts)
            .label(label("show.motion.start"))
            .selected(effect.start.clone())
            .on_change(move |value, _, cx| {
                let change = json!({ "start": value.to_string() });
                on_deck(&weak, cx, move |deck, cx| deck.send_effect(at, change, cx))
            });
        let duration = number(
            "show-effect-duration",
            label("show.motion.duration"),
            effect.duration,
            (0.01, 60.0),
        )
        .on_commit(dedupe(
            cx.entity(),
            effect_key("duration", slide, (at, effect)),
            effect.duration,
            move |deck, n, cx| deck.send_effect(at, json!({ "duration": n }), cx),
        ));
        let delay = number(
            "show-effect-delay",
            label("show.motion.delay"),
            effect.delay,
            (0.0, 60.0),
        )
        .on_commit(dedupe(
            cx.entity(),
            effect_key("delay", slide, (at, effect)),
            effect.delay,
            move |deck, n, cx| deck.send_effect(at, json!({ "delay": n }), cx),
        ));
        let (up, down, gone) = (
            send(json!({ "move": -1 })),
            send(json!({ "move": 1 })),
            send(json!({ "remove": true })),
        );
        let this = cx.entity();
        // The picked effect follows its move.
        let follow = |to: Option<usize>| {
            let this = this.clone();
            move |cx: &mut App| {
                this.update(cx, |panel, cx| {
                    panel.effect = to;
                    cx.notify();
                })
            }
        };
        let (went_up, went_down) = (follow(at.checked_sub(1)), follow(Some(at + 1)));
        let buttons = div()
            .flex()
            .gap_1()
            .child(
                Button::new("show-effect-up", label("show.motion.up"))
                    .size(ControlSize::Sm)
                    .disabled(at == 0)
                    .on_click(move |_, _, cx| {
                        up(cx);
                        went_up(cx)
                    }),
            )
            .child(
                Button::new("show-effect-down", label("show.motion.down"))
                    .size(ControlSize::Sm)
                    .disabled(at + 1 == count)
                    .on_click(move |_, _, cx| {
                        down(cx);
                        went_down(cx)
                    }),
            )
            .child(
                Button::new("show-effect-remove", label("show.motion.remove"))
                    .size(ControlSize::Sm)
                    .variant(ButtonVariant::Ghost)
                    .on_click(move |_, _, cx| {
                        gone(cx);
                        this.update(cx, |panel, cx| {
                            panel.effect = None;
                            cx.notify();
                        })
                    }),
            );
        rows.item(label("show.motion.start"), start)
            .item(label("show.motion.duration"), div().w_full().max_w_32().child(duration))
            .item(label("show.motion.delay"), div().w_full().max_w_32().child(delay))
            .item(SharedString::default(), buttons)
    }
}
