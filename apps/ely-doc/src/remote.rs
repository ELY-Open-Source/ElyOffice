//! Other programs read and edit open documents' text.

use ely_office::{prelude::*, remote};
use ely_platform::AppId;
use ely_ui::Ask;
use gpui::Context;

use crate::docs::Docs;

const USAGE: &str = "usage: ely-doc text docs
       ely-doc text get FILE
       ely-doc text replace FILE FIND WITH
       ely-doc text save FILE
get prints the body text, one paragraph per line; notes, comments and frames are left out.
replace changes every exact, case-matched FIND in the body, as one undo step.
ElyDoc must be running with FILE open.";

/// `ely-doc text ...` asks the running copy; its exit code.
pub fn command_line() -> Option<i32> {
    remote::command_line(AppId::Doc, "text", USAGE, request)
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

impl Docs {
    /// Answers a request; the kernel works off the window thread.
    pub fn answer_request(&mut self, ask: Ask, cx: &mut Context<Self>) {
        let Some((id, words, ask)) = self.asked(ask, cx) else {
            return;
        };
        match &words[..] {
            [verb, _] if verb == "get" => {
                let Some(copy) = self.tab(id).map(|tab| tab.work.join("text.fodt")) else {
                    return ask.answer(Err(format!("tab {id} closed")));
                };
                self.read_in(
                    id,
                    move |worker, doc| remote::body_text(&remote::copy::<ely_office::Lok>(worker, doc, &copy, "fodt")?),
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
                    move |worker, doc| worker.run(doc, "text.replace", &[&find, "false", "3", "0", "0", "1", &with, "true"]),
                    move |_, done, _, _| ask.answer(done.map(|()| String::new()).map_err(|error| format!("nothing replaced: {error}"))),
                    cx,
                );
            }
            _ => ask.answer(Err("unknown command".into())),
        }
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
