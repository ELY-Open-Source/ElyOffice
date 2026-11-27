//! Slide Show, Live Scoreboard: a sheet for each slide.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    forms::{Choice, Select},
    overlays::Dialog as Card,
    primitives::Severity,
};
use ely_office::{lok::Commands as _, prelude::*};
use gpui::{AnyElement, App, Context, IntoElement as _, ParentElement as _, Window};

use super::{Ask, Source, ask_table, clear::clear, rows, sheets};
use crate::deck::Deck;
use crate::dialog::{Dialog, edit};

/// The shown slide's choices; None shows nothing.
pub struct Picking {
    pub tab: u64,
    /// The slide's name, and its number from one.
    slide: (String, usize),
    found: Vec<Source>,
    picked: Option<usize>,
}

/// The choice that shows no scoreboard.
const NONE: &str = "none";

impl Deck {
    /// Asks ElyTable for its sheets, then offers them.
    pub fn ask_scoreboard(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.core.active() else {
            return;
        };
        let asked = self.ask_dialog();
        let read = cx.background_executor().spawn(async { sheets(ask_table) });
        cx.spawn(async move |this, cx| {
            let read = read.await;
            this.update(cx, |deck, cx| {
                if deck.dialog_due(asked, tab) {
                    deck.offer_scoreboard(tab, read, cx);
                }
            })
        })
        .detach();
    }

    fn offer_scoreboard(&mut self, tab: u64, read: std::io::Result<Vec<Source>>, cx: &mut Context<Self>) {
        let found = match read {
            Ok(found) => found,
            Err(error) => {
                log::warn!("show: ElyTable did not list its sheets: {error}");
                let text = Deck::text(cx, "show.score.no_table", &[("error", &error.to_string())]);
                return self.toast(text, Severity::Warning, cx);
            }
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
        let held = extra.scoreboards.get(&slide.name);
        if found.is_empty() && held.is_none() {
            let text = Deck::text(cx, "show.score.none_found", &[]);
            return self.toast(text, Severity::Warning, cx);
        }
        // The slide's sheet; else none once any is set.
        let picked = match held {
            Some(held) => found.iter().position(|source| held.is(source)),
            None => extra.scoreboards.is_empty().then_some(0),
        };
        let slide = (slide.name.clone(), current + 1);
        // Refused while another dialog is open; it says so.
        let _ = self.open_dialog(Dialog::Scoreboard(Picking { tab, slide, found, picked }), cx);
    }

    /// Keeps the pick for one slide, or for all.
    fn scoreboard_picked(&mut self, all: bool, cx: &mut Context<Self>) {
        let Some(Dialog::Scoreboard(Picking { tab, slide, found, picked })) = self.dialog.take() else {
            return;
        };
        let source = picked.map(|at| found[at].clone());
        let Some(extra) = self.tab_mut(tab).map(|tab| &mut tab.extra) else {
            return;
        };
        let names: Vec<String> = match all {
            true => extra.slides.list.iter().map(|slide| slide.name.clone()).collect(),
            false => vec![slide.0],
        };
        match source {
            Some(source) => self.link_scores(tab, names, source, ask_table, cx),
            None => {
                for name in names {
                    extra.scoreboards.remove(&name);
                }
                self.mark_unsaved(tab, cx);
                cx.notify();
            }
        }
    }

    /// Links the named slides; new links empty theirs.
    pub fn link_scores(&mut self, tab: u64, names: Vec<String>, source: Source, ask: Ask, cx: &mut Context<Self>) {
        let Some(extra) = self.tab(tab).map(|tab| &tab.extra) else {
            return;
        };
        // A linked slide keeps what was added since.
        let fresh: Vec<usize> = (extra.slides.list.iter().enumerate())
            .filter(|(_, slide)| names.contains(&slide.name) && !extra.scoreboards.contains_key(&slide.name))
            .map(|(at, _)| at)
            .collect();
        if fresh.is_empty() {
            return self.keep_link(tab, names, source, ask, cx);
        }
        log::info!("show: linking empties {} slides", fresh.len());
        self.write_in(
            tab,
            move |worker, doc| clear(worker, doc, &fresh),
            move |deck, cleared, _, cx| {
                // Cleared pictures and objects are read again.
                deck.repaint_all(tab, cx);
                deck.force_reread(tab, cx);
                match cleared {
                    Ok(()) => deck.keep_link(tab, names, source, ask, cx),
                    Err(error) => {
                        log::warn!("show: linking did not clear the slides: {error}");
                        let text = Deck::text(cx, "show.score.not_cleared", &[("error", &error.to_string())]);
                        deck.toast(text, Severity::Warning, cx);
                    }
                }
            },
            cx,
        );
    }

    /// Keeps the links, then says what the sheet gives.
    fn keep_link(&mut self, tab: u64, names: Vec<String>, source: Source, ask: Ask, cx: &mut Context<Self>) {
        let Some(extra) = self.tab_mut(tab).map(|tab| &mut tab.extra) else {
            return;
        };
        for name in names {
            extra.scoreboards.insert(name, source.clone());
        }
        self.mark_unsaved(tab, cx);
        cx.notify();
        let read = cx.background_executor().spawn(async move { rows(&source, ask) });
        cx.spawn(async move |this, cx| {
            let read = read.await;
            this.update(cx, |deck, cx| {
                let (text, severity) = match read {
                    Ok(rows) if rows.is_empty() => (Deck::text(cx, "show.score.blank", &[]), Severity::Warning),
                    Ok(rows) => (Deck::text(cx, "show.score.ready", &[("rows", &rows.len().to_string())]), Severity::Info),
                    Err(error) => (Deck::text(cx, "show.score.failed", &[("error", &error.to_string())]), Severity::Warning),
                };
                deck.toast(text, severity, cx);
            })
        })
        .detach();
    }

    pub fn scoreboard_dialog(&self, picking: &Picking, cx: &mut Context<Self>) -> AnyElement {
        let this = cx.entity();
        let label = |key: &str| Deck::text(cx, key, &[]);
        let sheets = (picking.found.iter().enumerate())
            .map(|(at, source)| Choice::new(at.to_string(), format!("{} — {}", source.name, source.sheet)));
        let choices: Vec<Choice> = std::iter::once(Choice::new(NONE, label("show.score.off"))).chain(sheets).collect();
        let pick = this.clone();
        let select = Select::new("show-score-sheet", choices)
            .label(label("show.score.pick"))
            .selected(picking.picked.map_or_else(|| NONE.to_string(), |at| at.to_string()))
            .on_change(move |value, _, cx| {
                // Only the NONE choice is not a number.
                let at = value.parse().ok();
                edit(&pick, cx, |dialog, _| {
                    if let Dialog::Scoreboard(picking) = dialog {
                        picking.picked = at;
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
        let title = Deck::text(cx, "show.score.title", &[("number", &picking.slide.1.to_string())]);
        let apply = |id: &'static str, key: &str, all: bool, variant| {
            let (this, label) = (this.clone(), label(key));
            move |_| {
                Button::new(id, label)
                    .variant(variant)
                    .on_click(move |_, _, cx| this.update(cx, |deck, cx| deck.scoreboard_picked(all, cx)))
            }
        };
        Card::new("show-score", title, dismiss)
            .detail(label("show.score.detail"))
            .child(DescriptionList::new().item(label("show.score.pick"), select))
            .action(ely_office::cancel(cx))
            .action(apply("show-score-all", "show.score.all", true, ButtonVariant::Secondary))
            .action(apply("show-score-this", "show.score.this", false, ButtonVariant::Primary))
            .into_any_element()
    }
}
