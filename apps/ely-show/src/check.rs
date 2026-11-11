//! What may go wrong in a show, and where (SHOW-14).

use ely_gpui_component::{
    lists::{ListItem, SelectableList},
    overlays::Dialog as Card,
    primitives::Severity,
};
use ely_lok::show::{Check, PageCheck, ShowSlide};
use ely_office::prelude::*;
use gpui::{AnyElement, App, Context, IntoElement, ParentElement as _, SharedString, Window, div};

use crate::deck::Deck;
use crate::dialog::Dialog;

/// What a finding is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Problem {
    Font,
    Media,
    /// A linked file; `missing` says if it is gone.
    Link,
    /// Something the kernel cannot show.
    Object,
}

/// One thing to fix, and where it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub problem: Problem,
    pub name: String,
    /// What draws a missing font.
    pub instead: Vec<String>,
    pub missing: bool,
    /// Slide indexes from zero.
    pub slides: Vec<usize>,
    pub masters: Vec<String>,
}

/// Every page's checks: slides by index, masters by name.
fn pages(check: &Check) -> impl Iterator<Item = (Result<usize, &str>, &PageCheck)> {
    let slides = check.slides.iter().enumerate().map(|(at, page)| (Ok(at), page));
    slides.chain(check.masters.iter().map(|(name, page)| (Err(name.as_str()), page)))
}

fn add(found: &mut Vec<Finding>, (problem, name, missing): (Problem, &str, bool), at: Result<usize, &str>) {
    let index = found.iter().position(|f| f.problem == problem && f.name == name);
    let finding = match index {
        Some(index) => &mut found[index],
        None => {
            found.push(Finding {
                problem,
                name: name.to_string(),
                instead: Vec::new(),
                missing,
                slides: Vec::new(),
                masters: Vec::new(),
            });
            found.last_mut().expect("just pushed")
        }
    };
    match at {
        Ok(slide) if !finding.slides.contains(&slide) => finding.slides.push(slide),
        Err(master) if !finding.masters.iter().any(|m| m == master) => {
            finding.masters.push(master.to_string())
        }
        _ => {}
    }
}

/// Missing fonts, missing media, links, then broken objects.
pub fn findings(check: &Check, slides: &[ShowSlide], missing_fonts: &[(String, Vec<String>)]) -> Vec<Finding> {
    let mut found = Vec::new();
    for (font, instead) in missing_fonts {
        let fonts = (Problem::Font, font.as_str(), true);
        let mut placed = false;
        for (at, _) in pages(check).filter(|(_, page)| page.fonts.contains(font)) {
            add(&mut found, fonts, at);
            placed = true;
        }
        if let Some(finding) = found.iter_mut().find(|f| f.problem == Problem::Font && f.name == *font) {
            finding.instead = instead.clone();
        }
        // Missing, yet no slide text asks for it.
        if !placed {
            found.push(Finding {
                problem: Problem::Font,
                name: font.clone(),
                instead: instead.clone(),
                missing: true,
                slides: Vec::new(),
                masters: Vec::new(),
            });
        }
    }
    for (at, slide) in slides.iter().enumerate() {
        for media in &slide.media {
            match (media.missing, media.link) {
                (true, _) => add(&mut found, (Problem::Media, &media.url, true), Ok(at)),
                (false, true) => add(&mut found, (Problem::Link, &media.url, false), Ok(at)),
                (false, false) => {}
            }
        }
    }
    for (at, page) in pages(check) {
        for (url, missing) in &page.links {
            add(&mut found, (Problem::Link, url, *missing), at);
        }
    }
    for (at, page) in pages(check) {
        for name in &page.broken {
            add(&mut found, (Problem::Object, name, true), at);
        }
    }
    found
}

impl Deck {
    /// Shows a finding's slide; master view numbers masters instead.
    pub(crate) fn go_to_finding(&mut self, part: usize, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        if tab.extra.masters {
            let text = Deck::text(cx, "show.masters.busy", &[]);
            return self.toast(text, Severity::Warning, cx);
        }
        let view = tab.view.clone();
        view.update(cx, |view, _| view.show_part(part));
        self.dialog = None;
        cx.notify();
    }

    /// Reads every slide and master, then lists what to fix.
    pub fn check_show(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            let text = Deck::text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        let (id, missing) = (tab.id, tab.view.read(cx).missing_fonts().to_vec());
        let asked = self.ask_dialog();
        log::info!("show: checks the deck");
        self.read_in(
            id,
            |worker, doc| {
                let check = ely_lok::show::check(&worker.values(doc, ely_lok::show::SLIDE_CHECK)?)?;
                let slides = ely_lok::show::slide_show(&worker.values(doc, ely_lok::show::SLIDE_SHOW)?)?;
                Ok((check, slides))
            },
            move |deck, read, _, cx| match read {
                Ok((check, slides)) if deck.dialog_due(asked, id) => {
                    let found = findings(&check, &slides, &missing);
                    log::info!("show: the check found {} things", found.len());
                    // Refused while another dialog is open; it says so.
                    let _ = deck.open_dialog(Dialog::Check(found), cx);
                }
                Ok(_) => log::info!("show: the check is no longer due"),
                Err(error) => deck.engine_failed(&format!("the check: {error}"), cx),
            },
            cx,
        );
    }

    /// The findings; a row goes to its first slide.
    pub fn check_dialog(&self, found: &[Finding], cx: &mut Context<Self>) -> AnyElement {
        let this = cx.entity();
        let text = |key: &str| Deck::text(cx, key, &[]);
        let closing = this.clone();
        let dismiss = move |_: &mut Window, cx: &mut App| {
            closing.update(cx, |deck, cx| {
                deck.dialog = None;
                cx.notify();
            })
        };
        let card = Card::new("show-check", text("show.check.title"), dismiss);
        if found.is_empty() {
            return card.child(div().child(text("show.check.clear"))).into_any_element();
        }
        let mut list = SelectableList::new("show-check-list").label(text("show.check.title"));
        for (at, finding) in found.iter().enumerate() {
            let kind = match (finding.problem, finding.missing) {
                (Problem::Font, _) => "show.check.font",
                (Problem::Media, _) => "show.check.media",
                (Problem::Link, true) => "show.check.link_missing",
                (Problem::Link, false) => "show.check.link",
                (Problem::Object, _) => "show.check.object",
            };
            let mut places: Vec<String> = (finding.slides.iter())
                .map(|slide| Deck::text(cx, "show.check.slide", &[("n", &(slide + 1).to_string())]).to_string())
                .collect();
            places.extend(finding.masters.iter().map(|name| {
                Deck::text(cx, "show.check.master", &[("name", name)]).to_string()
            }));
            let name = match (finding.name.is_empty(), finding.instead.is_empty()) {
                (true, _) => text("show.check.unnamed"),
                (false, true) => SharedString::from(finding.name.clone()),
                (false, false) => Deck::text(cx, "fonts.instead", &[("font", &finding.name), ("used", &finding.instead.join(", "))]),
            };
            let place = match places.is_empty() {
                true => text("show.check.nowhere").to_string(),
                false => places.join(", "),
            };
            let item = ListItem::new(SharedString::from(format!("show-check-{at}")), name)
                .description(format!("{} · {place}", text(kind)));
            list = list.row(SharedString::from(at.to_string()), item);
        }
        let firsts: Vec<Option<usize>> = found.iter().map(|f| f.slides.first().copied()).collect();
        let list = list.on_change(move |keys, _, cx| {
            let first = keys.first().and_then(|key| key.parse::<usize>().ok()).and_then(|at| firsts[at]);
            if let Some(part) = first {
                this.update(cx, |deck, cx| deck.go_to_finding(part, cx));
            }
        });
        card.child(list).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ely_lok::show::{Media, Transition};

    fn slide(media: Vec<Media>) -> ShowSlide {
        ShowSlide {
            hidden: false,
            notes: String::new(),
            transition: Transition {
                preset: String::new(),
                duration: 0.0,
                auto: false,
                time: 0.0,
            },
            effects: Vec::new(),
            media,
        }
    }

    fn media(url: &str, link: bool, missing: bool) -> Media {
        Media {
            index: 0,
            url: url.into(),
            link,
            file: url.into(),
            missing,
            repeat: false,
            bounds: (0, 0, 1, 1),
        }
    }

    #[test]
    fn findings_say_what_and_where_in_order() {
        let page = |fonts: &[&str], links: &[(&str, bool)], broken: &[&str]| PageCheck {
            fonts: fonts.iter().map(|f| f.to_string()).collect(),
            links: links.iter().map(|(u, m)| (u.to_string(), *m)).collect(),
            broken: broken.iter().map(|b| b.to_string()).collect(),
        };
        let check = Check {
            slides: vec![
                page(&["Lost", "Inter"], &[("file:///a.png", true)], &[]),
                page(&["Lost"], &[], &["Chart 1"]),
            ],
            masters: vec![("Plain".into(), page(&["Lost"], &[("file:///a.png", true)], &[]))],
        };
        let slides = [slide(vec![media("file:///clip.mp4", false, true)]), slide(vec![media("file:///b.mp3", true, false)])];
        let missing = [("Lost".to_string(), vec!["Inter".to_string()]), ("Gone".to_string(), vec![])];
        let found = findings(&check, &slides, &missing);
        assert_eq!(found[0].instead, ["Inter"], "the stand-in rides along");
        let short: Vec<(Problem, &str, Vec<usize>, Vec<String>)> = found
            .iter()
            .map(|f| (f.problem, f.name.as_str(), f.slides.clone(), f.masters.clone()))
            .collect();
        assert_eq!(
            short,
            [
                (Problem::Font, "Lost", vec![0, 1], vec!["Plain".to_string()]),
                (Problem::Font, "Gone", vec![], vec![]),
                (Problem::Media, "file:///clip.mp4", vec![0], vec![]),
                (Problem::Link, "file:///b.mp3", vec![1], vec![]),
                (Problem::Link, "file:///a.png", vec![0], vec!["Plain".to_string()]),
                (Problem::Object, "Chart 1", vec![1], vec![]),
            ]
        );
        assert!(found[4].missing && !found[3].missing);
    }
}
