//! The design panel: layout, master, theme, size (SHOW-01, 02).

use std::collections::HashMap;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    forms::{Choice, NumberInput, Select},
    layout::ScrollArea,
};
use ely_lok::show::LAYOUTS;
use ely_office::prelude::*;
use gpui::{
    AnyElement, App, Context, IntoElement, ParentElement as _, Render, SharedString, Styled as _,
    WeakEntity, Window, div,
};

use crate::deck::Deck;

/// Slide sizes in 1/100 mm.
const SIZES: [(&str, (i64, i64)); 5] = [
    ("wide", (33_867, 19_050)),
    ("hd", (28_000, 15_750)),
    ("standard", (25_400, 19_050)),
    ("a4", (29_700, 21_000)),
    ("letter", (27_940, 21_590)),
];

/// Twips to 1/100 mm.
pub fn mm100((w, h): (i64, i64)) -> (i64, i64) {
    ((w * 127 + 36) / 72, (h * 127 + 36) / 72)
}

/// The preset a 1/100 mm size matches, within 0.5 mm.
fn preset((w, h): (i64, i64)) -> Option<&'static str> {
    SIZES
        .iter()
        .find(|(_, (pw, ph))| (pw - w).abs() <= 50 && (ph - h).abs() <= 50)
        .map(|(key, _)| *key)
}

impl Deck {
    /// Runs a design command; the next read shows what changed.
    pub fn design_command(&mut self, id: &str, values: &[&str], cx: &mut Context<Self>) {
        let Some(tab) = self.core.active() else {
            return;
        };
        self.run_in(Some(tab), id, values, cx);
        self.force_reread(tab, cx);
    }

    /// Resizes the slides; the asked size shows until read back.
    pub(crate) fn resize(&mut self, (w, h): (i64, i64), cx: &mut Context<Self>) {
        let Some(tab) = self.core.active().and_then(|id| self.tab_mut(id)) else {
            return;
        };
        let (sw, sh) = tab.extra.shown_size();
        if (sw - w).abs() <= 2 && (sh - h).abs() <= 2 {
            log::info!("show: the slides already measure {w} x {h}");
            return;
        }
        let (id, view) = (tab.id, tab.view.clone());
        let before = view.read(cx).sent();
        tab.extra.size_asked = Some(((w, h), before + 1));
        let (sw, sh) = (w.to_string(), h.to_string());
        self.run_in(Some(id), "slide.size", &[&sw, &sh, "true"], cx);
        // Refused before sending: run_in said so already.
        if view.read(cx).sent() == before
            && let Some(tab) = self.tab_mut(id)
        {
            tab.extra.size_asked = None;
        }
        self.force_reread(id, cx);
    }

    /// Enters or leaves the master view.
    pub(crate) fn masters(&mut self, on: bool, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let id = tab.id;
        self.run_in(
            Some(id),
            if on { "master.edit" } else { "master.close" },
            &[],
            cx,
        );
        self.force_reread(id, cx);
    }
}

/// The right column over the active deck.
pub struct Panel {
    pub(crate) deck: WeakEntity<Deck>,
    /// Per field: value sent, value read, until the next read.
    pub(crate) sent: HashMap<String, (i64, Option<i64>)>,
    /// The tab and read the sends belong to.
    pub(crate) seen: Option<(u64, u64)>,
    /// The effect picked in the list.
    pub(crate) effect: Option<usize>,
}

impl Panel {
    pub fn new(deck: WeakEntity<Deck>, cx: &mut Context<Self>) -> Self {
        if let Some(deck) = deck.upgrade() {
            cx.observe(&deck, |_, _, cx| cx.notify()).detach();
        }
        Panel {
            deck,
            sent: HashMap::new(),
            seen: None,
            effect: None,
        }
    }

    /// The kernel's font families, once read.
    pub(crate) fn fonts(&self, cx: &App) -> Vec<SharedString> {
        self.deck
            .upgrade()
            .map(|deck| deck.read(cx).fonts.clone())
            .unwrap_or_default()
    }
}

/// Runs `then` on the deck behind `deck`.
pub(crate) fn on_deck(
    deck: &WeakEntity<Deck>,
    cx: &mut App,
    then: impl FnOnce(&mut Deck, &mut Context<Deck>),
) {
    if let Some(deck) = deck.upgrade() {
        deck.update(cx, then);
    }
}

impl Render for Panel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(deck) = self.deck.upgrade() else {
            return div();
        };
        let Some(objects) = deck
            .read(cx)
            .active_tab()
            .and_then(|tab| tab.extra.objects.clone())
        else {
            return div();
        };
        let top = match &objects.selection {
            Some(selection) => self.object_grid(selection, window, cx).into_any_element(),
            None => self.design_view(cx),
        };
        let list = (!objects.objects.is_empty()).then(|| {
            div().px_3().child(DescriptionList::new().stacked().item(
                Deck::text(cx, "show.objects", &[]),
                self.object_list(&objects.objects, cx),
            ))
        });
        div().size_full().child(
            ScrollArea::new("show-panel-scroll").vertical().size_full().child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .pb_3()
                    .child(top)
                    .children(self.motion_view(objects.selection.is_some(), cx))
                    .children(list),
            ),
        )
    }
}

impl Panel {
    /// Layout, master, theme and size of the shown slide.
    fn design_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let label = |key: &str| Deck::text(cx, key, &[]);
        let Some(deck) = self.deck.upgrade() else {
            return div().into_any_element();
        };
        let read = deck.read(cx);
        let Some(tab) = read.active_tab() else {
            return div().into_any_element();
        };
        let Some(design) = tab.extra.design.clone() else {
            return div().into_any_element();
        };
        let (masters, size) = (tab.extra.masters, tab.extra.shown_size());
        let weak = self.deck.clone();
        let theme = Select::new(
            "show-theme",
            design
                .themes
                .iter()
                .map(|name| Choice::new(name.clone(), name.clone())),
        )
        .label(label("show.design.theme"))
        .selected(design.theme.clone())
        .on_change({
            let weak = weak.clone();
            move |value, _, cx| {
                let value = value.to_string();
                on_deck(&weak, cx, |deck, cx| {
                    deck.design_command("slide.theme", &[&value], cx)
                })
            }
        });
        let mut rows = DescriptionList::new();
        if !masters {
            let mut layouts: Vec<Choice> = LAYOUTS
                .iter()
                .map(|n| Choice::new(n.to_string(), label(&format!("show.layout.{n}"))))
                .collect();
            if !LAYOUTS.contains(&design.layout) {
                layouts.push(Choice::new(
                    design.layout.to_string(),
                    label("show.layout.other"),
                ));
            }
            let layout = Select::new("show-layout", layouts)
                .label(label("show.design.layout"))
                .selected(design.layout.to_string())
                .on_change({
                    let weak = weak.clone();
                    move |value, _, cx| {
                        let value = value.to_string();
                        on_deck(&weak, cx, |deck, cx| {
                            deck.design_command("slide.layout", &[&value], cx)
                        })
                    }
                });
            let master = Select::new(
                "show-master",
                design
                    .masters
                    .iter()
                    .map(|name| Choice::new(name.clone(), name.clone())),
            )
            .label(label("show.design.master"))
            .selected(design.master.clone())
            .on_change({
                let weak = weak.clone();
                move |value, _, cx| {
                    let value = value.to_string();
                    on_deck(&weak, cx, |deck, cx| {
                        deck.design_command("slide.master", &[&value], cx)
                    })
                }
            });
            rows = rows
                .item(label("show.design.layout"), field(layout))
                .item(label("show.design.master"), field(master));
        }
        rows = rows.item(label("show.design.theme"), field(theme));
        if !masters {
            rows = size_rows(rows, &weak, size, cx);
        }
        let toggle = {
            let weak = weak.clone();
            let key = if masters {
                "show.masters.close"
            } else {
                "show.masters.edit"
            };
            Button::new("show-masters", label(key))
                .variant(ButtonVariant::Ghost)
                .on_click(move |_, _, cx| on_deck(&weak, cx, |deck, cx| deck.masters(!masters, cx)))
        };
        let insert = masters.then(|| {
            Button::new("show-master-new", label("show.masters.new"))
                .variant(ButtonVariant::Ghost)
                .on_click(move |_, _, cx| {
                    on_deck(&weak, cx, |deck, cx| {
                        deck.run_active("master.insert", &[], cx);
                        if let Some(id) = deck.core.active() {
                            deck.force_reread(id, cx);
                        }
                    })
                })
        });
        div()
            .size_full()
            .p_3()
            .flex()
            .flex_col()
            .gap_3()
            .child(rows)
            .child(div().flex().gap_2().child(toggle).children(insert))
            .into_any_element()
    }
}

fn field(control: impl IntoElement) -> gpui::Div {
    div().w_full().max_w_40().child(control)
}

/// The size preset, then width and height in centimetres.
fn size_rows(
    rows: DescriptionList,
    weak: &WeakEntity<Deck>,
    size: (i64, i64),
    cx: &App,
) -> DescriptionList {
    let label = |key: &str| Deck::text(cx, key, &[]);
    let current = preset(size).unwrap_or("custom");
    let mut sizes: Vec<Choice> = SIZES
        .iter()
        .map(|(key, _)| Choice::new(*key, label(&format!("show.size.{key}"))))
        .collect();
    if current == "custom" {
        sizes.push(Choice::new("custom", label("show.size.custom")));
    }
    let picking = weak.clone();
    let presets = Select::new("show-size", sizes)
        .label(label("show.design.size"))
        .selected(current)
        .on_change(move |value, _, cx| {
            let Some((_, size)) = SIZES.iter().find(|(key, _)| *key == value.as_ref()) else {
                return;
            };
            let size = *size;
            on_deck(&picking, cx, |deck, cx| deck.resize(size, cx))
        });
    let (w, h) = size;
    let side = |id: &'static str, value: i64, width: bool| {
        let weak = weak.clone();
        let name = label(if width {
            "show.design.width"
        } else {
            "show.design.height"
        });
        NumberInput::new(id, value as f64 / 1000.0)
            .label(name)
            .range(1.0, 300.0)
            .step(0.1)
            .precision(2)
            .on_commit(move |cm, _, cx| {
                let value = (cm * 1000.0).round() as i64;
                // Build on a size still on its way.
                on_deck(&weak, cx, |deck, cx| {
                    let Some((w, h)) = deck.active_tab().map(|tab| tab.extra.shown_size()) else {
                        return;
                    };
                    deck.resize(if width { (value, h) } else { (w, value) }, cx)
                })
            })
    };
    rows.item(label("show.design.size"), field(presets))
        .item(
            label("show.design.width"),
            div().w_full().max_w_32().child(side("show-width", w, true)),
        )
        .item(
            label("show.design.height"),
            div().w_full().max_w_32().child(side("show-height", h, false)),
        )
}

#[cfg(test)]
mod tests {
    #[test]
    fn sizes_match_presets_by_millimetre() {
        assert_eq!(super::preset(super::mm100((15875, 8930))), Some("hd"));
        assert_eq!(
            super::preset(super::mm100((14401, 10801))),
            Some("standard")
        );
        assert_eq!(super::preset((10000, 10000)), None);
        assert_eq!(super::mm100((14400, 10800)), (25400, 19050));
    }
}
