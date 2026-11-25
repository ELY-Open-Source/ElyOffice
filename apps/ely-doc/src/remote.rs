//! Other programs read and edit open documents' text.

use std::path::PathBuf;

use ely_lok::Pdf;
use ely_office::{
    prelude::*,
    remote::{self, Cli},
};
use ely_platform::AppId;
use ely_ui::Ask;
use gpui::Context;

use crate::docs::Docs;

mod format;
mod text;

const USAGE: &str = "usage: ely-doc docs
       ely-doc open FILE
       ely-doc new FILE
       ely-doc export FILE TARGET
       ely-doc save FILE
       ely-doc text get FILE [--json]
       ely-doc text replace FILE FIND WITH
       ely-doc text append FILE TEXT [--style STYLE]
       ely-doc format FILE FIND PROPERTY=VALUE ...
open and new start ElyDoc if needed; the rest require it running (with FILE open).
get prints the body text, one paragraph per line; notes, comments and frames are left out.
With --json, get prints a JSON array of {\"style\", \"text\"} objects.
replace changes every exact, case-matched FIND; newlines in WITH start new paragraphs (one undo).
append adds lines as paragraphs at the end (--style uses English name, e.g. Heading 1; one undo).
format styles every exact, case-matched FIND (one undo): bold, italic, underline (on|off), font,
size (1-999), color (#RRGGBB|none), style (English paragraph style for each matching paragraph).
Any argument written as - is read from standard input, up to 64 MiB.
Exit codes: 0 success, 1 failed, 2 wrong usage, 3 not running.";

/// Every page, as the export dialog starts.
const WHOLE: Pdf = Pdf {
    pages: None,
    comments: false,
    changes: true,
    handouts: 0,
};

/// `ely-doc open|new|export|text ...`; its exit code.
pub fn command_line() -> Option<i32> {
    remote::command_line(&Cli {
        app: AppId::Doc,
        nouns: &[("text", request), ("format", format::request)],
        usage: USAGE,
        exact: &[],
        blanks: &[("docx", include_bytes!("../../../packaging/windows/new/new.docx"))],
        exports: &["docx", "odt", "pdf"],
    })
}

/// The words sent; the file made absolute.
fn request(words: &[&str]) -> Result<Vec<String>, String> {
    Ok(match *words {
        ["docs"] => vec!["docs".into()],
        ["get", path] => vec!["get".into(), remote::absolute(path)?],
        ["get", path, "--json"] => vec!["get".into(), remote::absolute(path)?, "--json".into()],
        ["replace", path, find, with] => vec!["replace".into(), remote::absolute(path)?, find.into(), with.into()],
        ["append", path, text] => vec!["append".into(), remote::absolute(path)?, text.into(), String::new()],
        ["append", path, text, "--style", style] => vec!["append".into(), remote::absolute(path)?, text.into(), style.into()],
        ["save", path] => vec!["save".into(), remote::absolute(path)?],
        _ => return Err("unknown command".into()),
    })
}

impl Docs {
    /// Answers a request; the kernel works off the window thread.
    pub fn answer_request(&mut self, ask: Ask, cx: &mut Context<Self>) {
        let Some((id, words, ask)) = self.asked(ask, cx) else {
            return;
        };
        match &words[..] {
            [verb, _, rest @ ..] if verb == "get" && rest.len() <= 1 => {
                let Some(copy) = self.tab(id).map(|tab| tab.work.join("text.fodt")) else {
                    return ask.answer(Err(format!("tab {id} closed")));
                };
                let json = !rest.is_empty();
                self.read_in(
                    id,
                    move |worker, doc| {
                        let flat = remote::copy::<ely_office::Lok>(worker, doc, &copy, "fodt")?;
                        if !json {
                            return remote::body_text(&flat);
                        }
                        let paragraphs = remote::body_paragraphs(&flat)?;
                        let listed: Vec<_> = paragraphs.iter().map(|(style, text)| serde_json::json!({ "style": style, "text": text })).collect();
                        Ok(serde_json::Value::from(listed).to_string())
                    },
                    move |_, read, _, _| ask.answer(read.map_err(|error| error.to_string())),
                    cx,
                );
            }
            [verb, _, find, with] if verb == "replace" => {
                let (command, mode, find, with) = match text::breaks(find, with) {
                    Ok(None) => ("text.replace", "1", find.clone(), with.clone()),
                    Ok(Some((find, with))) => ("text.replace_breaks", "2", find, with),
                    Err(why) => return ask.answer(Err(why.into())),
                };
                self.write_in(
                    id,
                    move |worker, doc| worker.run(doc, command, &[&find, "false", "3", "0", "0", mode, &with, "true"]),
                    move |_, done, _, _| ask.answer(done.map(|()| String::new()).map_err(|error| format!("nothing replaced: {error}"))),
                    cx,
                );
            }
            [verb, _, lines, style] if verb == "append" => {
                let lines = match text::lines(lines) {
                    Ok(lines) => lines,
                    Err(why) => return ask.answer(Err(why.into())),
                };
                let style = style.clone();
                self.write_in(
                    id,
                    move |worker, doc| {
                        // The style is checked first, so nothing half-happens.
                        if let Some(why) = text::style_missing(worker, doc, &style)? {
                            return Ok(Err(why));
                        }
                        worker.run(doc, "text.append", &[&lines, &style]).map(Ok)
                    },
                    move |_, done, _, _| ask.answer(done.map_err(|error| error.to_string()).and_then(|done| done.map(|()| String::new()))),
                    cx,
                );
            }
            [verb, file, find, fields] if verb == "format" => {
                let nothing = format::NOTHING.replace("{file}", file).replace("{find}", find);
                let (find, fields) = (find.clone(), fields.clone());
                self.write_in(
                    id,
                    move |worker, doc| {
                        if let Some(why) = text::style_missing(worker, doc, format::style(&fields))? {
                            return Ok(Err(why));
                        }
                        // The style exists, so a refusal means no match.
                        Ok(worker.run(doc, "text.format", &[&find, &fields]).map_err(|error| {
                            log::info!("doc: another program's format took nothing: {error}");
                            nothing
                        }))
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
            return self.when_settled(id, move |docs, _, cx| docs.export_for(id, target, ask, cx));
        }
        let Some(work) = self.tab(id).map(|tab| tab.work.clone()) else {
            return ask.answer(Err(format!("tab {id} closed")));
        };
        let format = ely_office::files::save_as_format(&Self::KIND, &target);
        let named = remote::path_json(&target.display().to_string());
        self.read_in(
            id,
            move |worker, doc| {
                remote::export_to(&work, &target, |copy| {
                    let url = ely_lok::file_url(copy);
                    match format {
                        Some(format) => doc.save_as(worker, &url, format),
                        None => doc.export_pdf(worker, &url, &WHOLE),
                    }
                })
            },
            move |_, done, _, _| ask.answer(done.map(|()| named).map_err(|error| error.to_string())),
            cx,
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_words_carry_the_file_made_absolute() {
        let sent = super::request(&["replace", "a.docx", "cat", "dog"]).expect("a request");
        assert_eq!((sent[0].as_str(), sent[2].as_str(), sent[3].as_str()), ("replace", "cat", "dog"));
        assert!(std::path::Path::new(&sent[1]).is_absolute(), "{sent:?}");
        assert!(super::request(&["replace", "a.docx", "cat"]).is_err(), "too few words");
        assert!(super::request(&["set", "a.docx"]).is_err(), "no such verb");
    }
}
