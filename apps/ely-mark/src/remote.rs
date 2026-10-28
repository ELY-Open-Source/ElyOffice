//! Other programs read and edit open files' Markdown.

use ely_office::{prelude::*, remote};
use ely_platform::AppId;
use ely_ui::Ask;
use gpui::{App, Context, Entity};

use crate::desk::Desk;

const USAGE: &str = "usage: ely-mark text docs
       ely-mark text get FILE
       ely-mark text replace FILE FIND WITH
       ely-mark text save FILE
get prints the Markdown as it stands, unsaved typing included.
replace changes every exact, case-matched FIND in the Markdown, as one undo step.
ElyMark must be running with FILE open, not in preview.";

/// `ely-mark text ...` asks the running copy; its exit code.
pub fn command_line() -> Option<i32> {
    remote::command_line(AppId::Mark, "text", USAGE, request)
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

/// Requests from other programs reach `desk`.
pub fn listen(desk: &Entity<Desk>, cx: &mut App) {
    let asked = desk.clone();
    ely_ui::on_ask(cx, move |ask, cx| asked.update(cx, |desk, cx| desk.answer_request(ask, cx)));
}

impl Desk {
    /// Answers a request against the tab's document.
    pub fn answer_request(&mut self, ask: Ask, cx: &mut Context<Self>) {
        let Some((id, words, ask)) = self.asked(ask, cx) else {
            return;
        };
        let Some(document) = self.tab(id).map(|tab| tab.view.read(cx).document().clone()) else {
            return ask.answer(Err(format!("tab {id} closed")));
        };
        match &words[..] {
            [verb, _] if verb == "get" => ask.answer(Ok(document.read(cx).text(cx))),
            [verb, file, find, with] if verb == "replace" => {
                if find.is_empty() {
                    return ask.answer(Err("FIND must not be empty".into()));
                }
                let text = document.read(cx).text(cx);
                let edits: Vec<_> = (text.match_indices(find.as_str()))
                    .map(|(at, _)| (at..at + find.len(), with.clone()))
                    .collect();
                if edits.is_empty() {
                    return ask.answer(Err(format!("nothing replaced: {file} holds no {find:?}")));
                }
                let count = edits.len();
                if !document.update(cx, |document, cx| document.replace(edits, cx)) {
                    return ask.answer(Err(format!("{file} shows as a preview, which takes no edits")));
                }
                log::info!("mark: another program replaced {count} matches");
                ask.answer(Ok(String::new()));
            }
            _ => ask.answer(Err("unknown command".into())),
        }
    }
}

#[cfg(test)]
mod tests;
