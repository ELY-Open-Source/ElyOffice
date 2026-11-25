//! Other programs read and edit open decks' text.

use std::path::PathBuf;

use ely_office::{
    prelude::*,
    remote::{self, Cli},
};
use ely_platform::AppId;
use ely_ui::Ask;
use gpui::Context;

use crate::deck::Deck;

mod slide;

const USAGE: &str = "usage: ely-show docs
       ely-show open FILE
       ely-show new FILE
       ely-show export FILE TARGET
       ely-show save FILE
       ely-show text get FILE
       ely-show text replace FILE FIND WITH
       ely-show slide list FILE
       ely-show slide add FILE [POSITION] [--layout NAME]
       ely-show slide duplicate FILE N
       ely-show slide delete FILE N
       ely-show slide move FILE N TO
open and new start ElyShow if needed; the rest require it running (with FILE open).
text get prints slides (form feed between; no notes/comments); replace is exact (1 undo/text box).
slide: list prints JSON; add/duplicate/delete/move are 1 undo (view stays; POSITION defaults last).
layouts (default title-content): blank, title, title-content, title-2-content, title-only,
centered-text, title-content-2-content, title-2-content-content, title-content-over-content,
title-2-content-over-content, title-4-content, title-6-content.
Any argument written as - is read from standard input, up to 64 MiB.
Exit codes: 0 success, 1 failed, 2 wrong usage, 3 not running.";

/// `ely-show open|new|export|text ...`; its exit code.
pub fn command_line() -> Option<i32> {
    remote::command_line(&Cli {
        app: AppId::Show,
        nouns: &[("text", request), ("slide", slide::request)],
        usage: USAGE,
        exact: &[],
        blanks: &[("pptx", include_bytes!("../../../packaging/windows/new/new.pptx"))],
        exports: &["pptx", "odp", "pdf"],
    })
}

/// The words sent; the file made absolute.
fn request(words: &[&str]) -> Result<Vec<String>, String> {
    Ok(match *words {
        ["docs"] => vec!["docs".into()],
        ["get", path] => vec!["get".into(), remote::absolute(path)?],
        ["replace", path, find, with] => vec!["replace".into(), remote::absolute(path)?, find.into(), with.into()],
        ["save", path] => vec!["save".into(), remote::absolute(path)?],
        _ => return Err("unknown command".into()),
    })
}

impl Deck {
    /// Answers a request; the kernel works off the window thread.
    pub fn answer_request(&mut self, ask: Ask, cx: &mut Context<Self>) {
        let Some((id, words, ask)) = self.asked(ask, cx) else {
            return;
        };
        match &words[..] {
            [verb, _] if verb == "get" => {
                let Some(copy) = self.tab(id).map(|tab| tab.work.join("text.fodp")) else {
                    return ask.answer(Err(format!("tab {id} closed")));
                };
                self.read_in(
                    id,
                    move |worker, doc| remote::slide_text(&remote::copy::<ely_office::Lok>(worker, doc, &copy, "fodp")?),
                    move |_, read, _, _| ask.answer(read.map_err(|error| error.to_string())),
                    cx,
                );
            }
            [verb, _, find, with] if verb == "replace" => {
                if find.is_empty() || [find, with].iter().any(|text| text.contains(['\n', '\r'])) {
                    return ask.answer(Err("FIND must be text within one paragraph, and WITH too".into()));
                }
                let (find, with) = (find.clone(), with.clone());
                // A box being typed in: where its caret shows.
                let caret = self.tab(id).map(|tab| tab.view.read(cx)).filter(|view| view.caret()).and_then(|view| view.cursor());
                self.write_in(
                    id,
                    move |worker, doc| replace_all(worker, doc, &find, &with, caret),
                    move |_, done, _, _| ask.answer(done.map(|()| String::new()).map_err(|error| error.to_string())),
                    cx,
                );
            }
            [verb, _] if verb == "slide.list" => self.read_in(
                id,
                move |worker, doc| Ok(slide::listed(&ely_lok::show::slides(&worker.values(doc, ely_lok::show::SLIDES)?)?)),
                move |_, read, _, _| ask.answer(read.map_err(|error| error.to_string())),
                cx,
            ),
            [verb, _, numbers @ ..] if verb.starts_with("slide.") => {
                let (verb, numbers) = (verb["slide.".len()..].to_owned(), numbers.to_vec());
                self.write_in(
                    id,
                    move |worker, doc| {
                        // Numbers are checked against the deck as it stands.
                        let count = ely_lok::show::slides(&worker.values(doc, ely_lok::show::SLIDES)?)?.len();
                        let [verb, at, value] = match slide::checked(&verb, &numbers, count) {
                            Ok(sent) => sent,
                            Err(why) => return Ok(Err(why)),
                        };
                        // A move onto itself changes nothing.
                        if verb == "move" && at == value {
                            return Ok(Ok(()));
                        }
                        worker.run(doc, "slide.at", &[&verb, &at, &value]).map(Ok)
                    },
                    move |_, done, _, _| ask.answer(done.map_err(|error| error.to_string()).and_then(|done| done.map(|()| String::new()))),
                    cx,
                );
            }
            [verb, _, target] if verb == "export" => self.export_for(id, PathBuf::from(target), ask, cx),
            _ => ask.answer(Err("unknown command".into())),
        }
    }

    /// A copy at `target` by its extension; the tab stays.
    fn export_for(&mut self, id: u64, target: PathBuf, ask: Ask, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |deck, _, cx| deck.export_for(id, target, ask, cx));
        }
        let Some(work) = self.tab(id).map(|tab| tab.work.clone()) else {
            return ask.answer(Err(format!("tab {id} closed")));
        };
        let format = ely_office::files::save_as_format(&Self::KIND, &target);
        let named = remote::path_json(&target.display().to_string());
        self.read_in(
            id,
            move |worker, doc| {
                remote::export_to(&work, &target, |copy| match format {
                    Some(format) => doc.save_as(worker, &ely_lok::file_url(copy), format),
                    None => crate::output::whole_pdf(worker, doc, &work.join("export.odp"), &ely_lok::file_url(copy)),
                })
            },
            move |_, done, _, _| ask.answer(done.map(|()| named).map_err(|error| error.to_string())),
            cx,
        );
    }
}

/// Impress narrows Replace All to marked objects; unmark, then restore.
pub(crate) fn replace_all(worker: &mut ely_lok::Worker, doc: &ely_lok::Doc, find: &str, with: &str, caret: Option<ely_lok::Rect>) -> std::io::Result<()> {
    let read = |worker: &mut ely_lok::Worker, doc: &ely_lok::Doc| {
        ely_lok::show::objects(&worker.values(doc, ely_lok::show::OBJECTS)?)
    };
    let shown = read(worker, doc)?;
    let editing = shown.selection.as_ref().is_some_and(|selection| selection.editing);
    let before = shown.objects;
    let marked: Vec<String> =
        before.iter().enumerate().filter(|(_, object)| object.selected).map(|(at, _)| at.to_string()).collect();
    // Marks inside a group have no index to restore.
    if shown.selection.as_ref().map_or(0, |selection| selection.count) != marked.len() {
        return Err(std::io::Error::other("an object inside a group is selected in ElyShow; leave the group first"));
    }
    if !marked.is_empty() {
        worker.run(doc, "object.deselect", &["false"])?;
    }
    let replaced = worker.run(doc, "text.replace", &[find, "false", "3", "0", "0", "1", with, "true"]);
    if !marked.is_empty() {
        let now = worker.now(doc)?;
        let same = |a: &[ely_lok::show::Object], b: &[ely_lok::show::Object]| {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| (&a.kind, &a.placeholder) == (&b.kind, &b.placeholder))
        };
        if !same(&before, &read(worker, &now)?.objects) {
            return Err(std::io::Error::other("the slide's objects changed; the selection was not restored"));
        }
        let picks: Vec<[&str; 2]> =
            marked.iter().enumerate().map(|(n, at)| [at.as_str(), if n > 0 { "true" } else { "false" }]).collect();
        let commands: Vec<(&str, &[&str])> = picks.iter().map(|values| ("object.select", values.as_slice())).collect();
        worker.run_all(&now, &commands)?;
        // Unmarking ended the edit; a click reopens it.
        match (caret, marked.as_slice()) {
            // The view's caret may outlive an Escape.
            (Some((x, y, width, height)), [_]) if editing => {
                log::info!("show: the text edit a replace ended opens again at its caret");
                for kind in [0, 1] {
                    let at = ely_lok::Mouse { kind, x: x + width / 2, y: y + height / 2, count: 1, buttons: 1, modifiers: 0 };
                    worker.post_mouse(&now, at)?;
                }
            }
            // Just entered: its caret, not yet told, is the end.
            (None, [at]) if editing => {
                log::info!("show: the text edit a replace ended opens again at its end");
                let marked = worker.now(doc)?;
                worker.run(&marked, "object.edit", &[at.as_str(), "false", "true"])?;
            }
            _ => {}
        }
    }
    replaced.map_err(|error| std::io::Error::other(format!("nothing replaced: {error}")))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_words_carry_the_file_made_absolute() {
        let sent = super::request(&["replace", "a.pptx", "cat", "dog"]).expect("a request");
        assert_eq!((sent[0].as_str(), sent[2].as_str(), sent[3].as_str()), ("replace", "cat", "dog"));
        assert!(std::path::Path::new(&sent[1]).is_absolute(), "{sent:?}");
        assert!(super::request(&["replace", "a.pptx", "cat"]).is_err(), "too few words");
        assert!(super::request(&["set", "a.pptx"]).is_err(), "no such verb");
    }
}
