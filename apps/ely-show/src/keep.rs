//! Countdowns and scoreboards travel inside the saved deck.

use std::{io, path::Path, time::Duration};

use ely_gpui_component::primitives::Severity;
use ely_office::{Finish, prelude::*};
use gpui::{AppContext as _, Context};

use crate::deck::Deck;
use crate::extra::Extra;
use crate::slideshow::scoreboard::{Ask, Source, ask_table, renamed};
use crate::slideshow::timer::{Clocks, Spot, Timer};

mod package;

/// What one slide keeps, by its place in the deck.
#[derive(Clone, Debug, PartialEq)]
pub struct Held {
    pub at: usize,
    pub score: Option<Source>,
    pub timer: Option<Timer>,
}

/// A tab's kept data, beside what its file holds.
#[derive(Default)]
pub struct Kept {
    /// Read from the file; its slides are not listed yet.
    waiting: bool,
    /// As last seen over the normal slides.
    latest: Vec<Held>,
    saved: Vec<Held>,
}

/// The formats that are zip packages ElyShow can mark.
const MARKED: [&str; 2] = ["pptx", "odp"];

/// Places loaded data on slides; returns what is held.
fn sync(extra: &mut Extra) -> &[Held] {
    // Master pages replace the slide list meanwhile.
    if !extra.masters && !extra.slides.list.is_empty() {
        let names = extra.slides.list.iter().map(|slide| slide.name.clone());
        if std::mem::take(&mut extra.kept.waiting) {
            for (at, name) in names.clone().enumerate() {
                let Some(held) = extra.kept.latest.iter().find(|held| held.at == at) else {
                    continue;
                };
                if let Some(score) = &held.score {
                    extra.scoreboards.insert(name.clone(), score.clone());
                }
                if let Some(timer) = held.timer {
                    extra.timers.insert(name, timer);
                }
            }
        }
        extra.kept.latest = names
            .enumerate()
            .filter_map(|(at, name)| {
                let score = extra.scoreboards.get(&name).cloned();
                let timer = extra.timers.get(&name).copied();
                (score.is_some() || timer.is_some()).then_some(Held { at, score, timer })
            })
            .collect();
    }
    &extra.kept.latest
}

impl Deck {
    /// Reads what the opened file of tab `id` keeps.
    pub fn read_kept(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(path) = self.tab(id).and_then(|tab| tab.session.path()).map(Path::to_path_buf) else {
            return;
        };
        // Only the file's tail is read, before any save.
        let held = match package::read(&path) {
            Ok(Some(held)) => held,
            Ok(None) => return,
            Err(error) => {
                log::warn!("show: tab {id}'s countdowns and scoreboards did not read: {error}");
                let text = Deck::text(cx, "show.keep.unread", &[("error", &error.to_string())]);
                return self.toast(text, Severity::Warning, cx);
            }
        };
        log::info!("show: tab {id}'s file keeps {} slides' countdowns or scoreboards", held.len());
        let last = held.iter().filter_map(|held| held.timer).map(|timer| timer.id).max();
        if let Some(last) = last {
            let clocks = self.clocks(id, cx);
            clocks.update(cx, |clocks, _| clocks.taken(last));
        }
        if let Some(tab) = self.tab_mut(id) {
            tab.extra.kept = Kept { waiting: true, latest: held.clone(), saved: held };
        }
    }

    /// Tab `id`'s clocks, made at first need.
    pub fn clocks(&mut self, id: u64, cx: &mut Context<Self>) -> gpui::Entity<Clocks> {
        if let Some(clocks) = self.tab(id).and_then(|tab| tab.extra.clocks.clone()) {
            return clocks;
        }
        let clocks = cx.new(|_| Clocks::default());
        cx.observe(&clocks, |_, _, cx| cx.notify()).detach();
        if let Some(tab) = self.tab_mut(id) {
            tab.extra.clocks = Some(clocks.clone());
        }
        clocks
    }

    /// Applies what waits for tab `id`'s slides.
    pub fn place_kept(&mut self, id: u64) {
        if let Some(tab) = self.tab_mut(id) {
            sync(&mut tab.extra);
        }
    }

    /// A change of ElyShow's own leaves the deck unsaved.
    pub fn mark_unsaved(&mut self, id: u64, cx: &mut Context<Self>) {
        if let Some(tab) = self.tab_mut(id) {
            sync(&mut tab.extra);
            if tab.extra.kept.latest != tab.extra.kept.saved {
                tab.session.edit();
                cx.notify();
            }
        }
    }

    /// The kernel's undo to saved does not undo ours.
    pub fn stay_unsaved(&mut self, id: u64, cx: &mut Context<Self>) {
        let saved = self.tab(id).is_some_and(|tab| tab.session.state() == ely_files::SaveState::Saved);
        if saved {
            self.mark_unsaved(id, cx);
        }
    }

    /// What saving tab `id` adds to the bytes.
    pub fn keeping(&mut self, id: u64, format: &str, ask: Ask, cx: &mut Context<Self>) -> Option<Finish> {
        let tab = self.tab_mut(id)?;
        let held = sync(&mut tab.extra).to_vec();
        tab.extra.kept.saved = held.clone();
        if held.is_empty() {
            return None;
        }
        if !MARKED.contains(&format) {
            let text = Deck::text(cx, "show.keep.format", &[("format", format)]);
            self.toast(text, Severity::Warning, cx);
            return None;
        }
        Some(Box::new(move |bytes| package::seal(bytes, &renamed(held, ask))))
    }

    /// [`Deck::keeping`], asking the running ElyTable.
    pub fn finish_save(&mut self, id: u64, format: &str, cx: &mut Context<Self>) -> Option<Finish> {
        self.keeping(id, format, ask_table, cx)
    }
}

/// The words a spot is kept as.
const SPOTS: [(Spot, &str); 4] = [
    (Spot::Top, "top"),
    (Spot::Center, "center"),
    (Spot::Bottom, "bottom"),
    (Spot::Corner, "corner"),
];

fn invalid(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("unreadable kept {what}"))
}

fn written(held: &[Held]) -> serde_json::Value {
    let slides: Vec<_> = held
        .iter()
        .map(|held| {
            let score = held.score.as_ref().map(|score| {
                serde_json::json!({ "path": score.path, "name": score.name, "sheet": score.sheet })
            });
            let timer = held.timer.map(|timer| {
                let spot = SPOTS.iter().find(|(spot, _)| *spot == timer.spot).map(|(_, word)| *word);
                serde_json::json!({ "id": timer.id, "seconds": timer.length.as_secs(), "spot": spot, "at": timer.at })
            });
            serde_json::json!({ "slide": held.at, "score": score, "timer": timer })
        })
        .collect();
    serde_json::json!({ "version": 1, "slides": slides })
}

fn parsed(value: &serde_json::Value) -> io::Result<Vec<Held>> {
    if value["version"] != 1 {
        return Err(invalid("version"));
    }
    let slides = value["slides"].as_array().ok_or_else(|| invalid("slides"))?;
    slides.iter().map(slide).collect()
}

fn slide(value: &serde_json::Value) -> io::Result<Held> {
    let at = value["slide"].as_u64().ok_or_else(|| invalid("slide"))? as usize;
    let score = match &value["score"] {
        serde_json::Value::Null => None,
        score => {
            let word = |key: &str| score[key].as_str().map(str::to_owned).ok_or_else(|| invalid("scoreboard"));
            let path = match &score["path"] {
                serde_json::Value::Null => None,
                _ => Some(word("path")?),
            };
            // ElyTable's key dies with it: found again.
            Some(Source { book: String::new(), path, name: word("name")?, sheet: word("sheet")? })
        }
    };
    let timer = match &value["timer"] {
        serde_json::Value::Null => None,
        timer => {
            let number = |key: &str| timer[key].as_u64().ok_or_else(|| invalid("countdown"));
            let spot = SPOTS.iter().find(|(_, word)| timer["spot"] == *word).map(|(spot, _)| *spot);
            let at = match &timer["at"] {
                serde_json::Value::Null => None,
                at => {
                    let share = |n: usize| at[n].as_f64().filter(|share| (0.0..=1.0).contains(share));
                    let (x, y) = share(0).zip(share(1)).ok_or_else(|| invalid("countdown place"))?;
                    Some((x as f32, y as f32))
                }
            };
            Some(Timer {
                id: number("id")?,
                length: Duration::from_secs(number("seconds")?),
                spot: spot.ok_or_else(|| invalid("countdown spot"))?,
                at,
            })
        }
    };
    Ok(Held { at, score, timer })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_written_reads_back_without_elytables_key() {
        let score = Source { book: "7:0".into(), path: None, name: "Game".into(), sheet: "S 1".into() };
        let timer = Timer { id: 3, length: Duration::from_secs(90), spot: Spot::Corner, at: Some((0.25, 0.5)) };
        let held = vec![
            Held { at: 0, score: Some(score.clone()), timer: None },
            Held { at: 4, score: None, timer: Some(timer) },
        ];
        let read = parsed(&written(&held)).expect("reads");
        assert_eq!(read[0].score, Some(Source { book: String::new(), ..score }));
        assert_eq!(read[1], held[1]);
        for (spot, _) in SPOTS {
            let held = [Held { at: 0, score: None, timer: Some(Timer { spot, ..timer }) }];
            assert_eq!(parsed(&written(&held)).expect("reads"), held);
        }
        let mut broken = written(&held);
        broken["slides"][1]["timer"]["spot"] = "middle".into();
        assert!(parsed(&broken).is_err(), "refused, not guessed");
    }
}
