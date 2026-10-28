//! Other programs read and edit open decks' text.

use ely_office::{prelude::*, remote};
use ely_platform::AppId;
use ely_ui::Ask;
use gpui::Context;

use crate::deck::Deck;

const USAGE: &str = "usage: ely-show text docs
       ely-show text get FILE
       ely-show text replace FILE FIND WITH
       ely-show text save FILE
get prints each slide's text, one paragraph per line, a form feed between slides; notes and comments are left out.
replace changes every exact, case-matched FIND on the slides; as in Impress, each text box changed is one undo step.
ElyShow must be running with FILE open.";

/// `ely-show text ...` asks the running copy; its exit code.
pub fn command_line() -> Option<i32> {
    remote::command_line(AppId::Show, "text", USAGE, request, &[])
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
                self.read_in(
                    id,
                    move |worker, doc| replace_all(worker, doc, &find, &with),
                    move |_, done, _, _| ask.answer(done.map(|()| String::new()).map_err(|error| error.to_string())),
                    cx,
                );
            }
            _ => ask.answer(Err("unknown command".into())),
        }
    }
}

/// Impress narrows Replace All to marked objects; unmark, then restore.
fn replace_all(worker: &mut ely_lok::Worker, doc: &ely_lok::Doc, find: &str, with: &str) -> std::io::Result<()> {
    let read = |worker: &mut ely_lok::Worker, doc: &ely_lok::Doc| {
        Ok::<_, std::io::Error>(ely_lok::show::objects(&worker.values(doc, ely_lok::show::OBJECTS)?)?)
    };
    let shown = read(worker, doc)?;
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
