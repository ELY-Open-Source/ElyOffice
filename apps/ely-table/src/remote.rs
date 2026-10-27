//! Other programs read and write open workbooks (GitHub #19).

use ely_lok::sheet;
use ely_office::{prelude::*, remote};
use ely_platform::AppId;
use ely_ui::Ask;
use gpui::Context;

use crate::book::Book;

const USAGE: &str = "usage: ely-table cell docs
       ely-table cell get FILE ADDRESS
       ely-table cell set FILE ADDRESS TEXT
       ely-table cell save FILE
ADDRESS names its sheet, as in Sheet1!A1. ElyTable must be running with FILE open.";

/// `ely-table cell ...` asks the running copy; its exit code.
pub fn command_line() -> Option<i32> {
    remote::command_line(AppId::Table, "cell", USAGE, request)
}

/// The words sent; the file made absolute.
fn request(words: &[&str]) -> Result<Vec<String>, String> {
    Ok(match *words {
        ["docs"] => vec!["docs".into()],
        ["get", path, address] => vec!["get".into(), remote::absolute(path)?, address.into()],
        ["set", path, address, text] => vec!["set".into(), remote::absolute(path)?, address.into(), text.into()],
        ["save", path] => vec!["save".into(), remote::absolute(path)?],
        _ => return Err("unknown command".into()),
    })
}

impl Book {
    /// Answers a request; the kernel works off the window thread.
    pub fn answer(&mut self, ask: Ask, cx: &mut Context<Self>) {
        let Some((id, words, ask)) = self.asked(ask, cx) else {
            return;
        };
        let failed = |error: std::io::Error| error.to_string();
        match &words[..] {
            [verb, _, address] if verb == "get" => {
                let query = sheet::cell_at_query(address);
                self.read_in(
                    id,
                    move |worker, doc| sheet::cell_at(&worker.values(doc, &query)?),
                    move |_, read, _, _| {
                        let shown = |cell: sheet::CellAt| serde_json::json!({ "address": cell.address, "text": cell.text, "input": cell.input }).to_string();
                        ask.answer(read.map(shown).map_err(failed))
                    },
                    cx,
                );
            }
            [verb, _, address, text] if verb == "set" => {
                let (address, text) = (address.clone(), text.clone());
                self.read_in(
                    id,
                    move |worker, doc| worker.run(doc, "cell.set", &[&address, &text]),
                    move |_, done, _, _| ask.answer(done.map(|()| String::new()).map_err(failed)),
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
        let sent = super::request(&["get", "a.xlsx", "S!A1"]).expect("a request");
        assert_eq!((sent[0].as_str(), sent[2].as_str()), ("get", "S!A1"));
        assert!(std::path::Path::new(&sent[1]).is_absolute(), "{sent:?}");
        assert!(super::request(&["get", "a.xlsx"]).is_err(), "too few words");
        assert!(super::request(&["drop", "a.xlsx"]).is_err(), "no such verb");
    }
}
