//! Other programs read and edit open files' Markdown.

use std::path::PathBuf;

use ely_office::{
    prelude::*,
    remote::{self, Cli},
};
use ely_platform::AppId;
use ely_ui::Ask;
use gpui::{App, Context, Entity};

use crate::desk::Desk;
use crate::output::{self, Format, Target};

const USAGE: &str = "usage: ely-mark docs
       ely-mark open FILE
       ely-mark new FILE
       ely-mark export FILE TARGET
       ely-mark save FILE
       ely-mark text get FILE
       ely-mark text replace FILE FIND WITH
       ely-mark text set FILE TEXT
       ely-mark text append FILE TEXT
open and new start ElyMark if needed; the rest require it running (with FILE open, not in preview for text).
get prints the Markdown as it stands, byte for byte, unsaved typing included.
replace changes every exact, case-matched FIND in the Markdown, as one undo step.
set replaces the whole Markdown (empty clears it; keeps file line endings; one undo step).
append adds non-empty TEXT on a new line at the end (keeps file line endings; one undo step).
Any argument written as - is read from standard input, up to 64 MiB.
Exit codes: 0 success, 1 failed, 2 wrong usage, 3 not running.";
const APPEND_EMPTY: &str = "TEXT must not be empty";
const PREVIEW: &str = "{file} shows as a preview, which takes no edits";

/// `ely-mark open|new|export|text ...`; its exit code.
pub fn command_line() -> Option<i32> {
    remote::command_line(&Cli {
        app: AppId::Mark,
        nouns: &[("text", request)],
        usage: USAGE,
        exact: &["get"],
        blanks: &[("md", include_bytes!("../../../packaging/windows/new/new.md")), ("markdown", b"")],
        exports: &["html", "pdf", "txt"],
    })
}

/// The words sent; the file made absolute.
fn request(words: &[&str]) -> Result<Vec<String>, String> {
    Ok(match *words {
        ["docs"] => vec!["docs".into()],
        ["get", path] => vec!["get".into(), remote::absolute(path)?],
        ["replace", path, find, with] => vec!["replace".into(), remote::absolute(path)?, find.into(), with.into()],
        ["save", path] => vec!["save".into(), remote::absolute(path)?],
        ["set", path, text] => vec!["set".into(), remote::absolute(path)?, text.into()],
        ["append", _, ""] => return Err(APPEND_EMPTY.into()),
        ["append", path, text] => vec!["append".into(), remote::absolute(path)?, text.into()],
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
                    return ask.answer(Err(PREVIEW.replace("{file}", file)));
                }
                log::info!("mark: another program replaced {count} matches");
                ask.answer(Ok(String::new()));
            }
            [verb, file, words] if verb == "set" || verb == "append" => {
                // The editor's text holds LF; saving restores the file's.
                let words = words.replace("\r\n", "\n").replace('\r', "\n");
                let text = document.read(cx).text(cx);
                let edit = match verb.as_str() {
                    "set" => (0..text.len(), words),
                    _ if text.is_empty() || text.ends_with('\n') => (text.len()..text.len(), words),
                    _ => (text.len()..text.len(), format!("\n{words}")),
                };
                if !document.update(cx, |document, cx| document.replace(vec![edit], cx)) {
                    return ask.answer(Err(PREVIEW.replace("{file}", file)));
                }
                log::info!("mark: another program's {verb} changed the text");
                ask.answer(Ok(String::new()));
            }
            [verb, _, target] if verb == "export" => self.export_for(id, PathBuf::from(target), ask, cx),
            _ => ask.answer(Err("unknown command".into())),
        }
    }

    /// Writes the tab to `target`, typed by its extension.
    fn export_for(&mut self, id: u64, target: PathBuf, ask: Ask, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |desk, _, cx| desk.export_for(id, target, ask, cx));
        }
        let (Some(source), Some(markdown)) = (self.tab(id), self.text_of(id)) else {
            return ask.answer(Err(format!("tab {id} closed")));
        };
        let format = match target.extension().map(|ext| ext.to_string_lossy().to_lowercase()).as_deref() {
            Some("pdf") => Format::Pdf,
            Some("txt") => Format::Text,
            _ => Format::Page,
        };
        let session = source.session.path().map(PathBuf::from);
        let (revision, work) = (source.session.revision(), source.work.clone());
        cx.spawn(async move |_, cx| {
            let named = remote::path_json(&target.display().to_string());
            let written = cx
                .background_executor()
                .spawn(async move {
                    let beside = session.as_deref().and_then(|path| path.parent());
                    let to = Target {
                        path: &target,
                        session: session.as_deref(),
                        revision,
                        work: &work,
                    };
                    output::write(format, &markdown, beside, &to)
                })
                .await;
            ask.answer(written.map(|_| named).map_err(|error| error.to_string()));
        })
        .detach();
    }
}

#[cfg(test)]
mod tests;
