//! Other programs read and write open workbooks (GitHub #19).

use std::path::PathBuf;

use ely_lok::sheet;
use ely_office::{
    prelude::*,
    remote::{self, Cli},
};
use ely_platform::AppId;
use ely_ui::Ask;
use gpui::Context;

use crate::book::Book;

const USAGE: &str = "usage: ely-table docs
       ely-table open FILE
       ely-table new FILE
       ely-table export FILE TARGET
       ely-table save FILE
       ely-table cell get FILE ADDRESS
       ely-table cell set FILE ADDRESS TEXT
open and new start ElyTable if needed; the rest require it running (with FILE open).
ADDRESS names its sheet, as in Sheet1!A1. cell get prints JSON; cell set writes TEXT (= for a formula) as one undo step.
Any argument written as - is read from standard input, up to 64 MiB.
Exit codes: 0 success, 1 failed, 2 wrong usage, 3 not running.";

/// `ely-table open|new|export|cell ...`; its exit code.
pub fn command_line() -> Option<i32> {
    remote::command_line(&Cli {
        app: AppId::Table,
        noun: "cell",
        usage: USAGE,
        request,
        exact: &[],
        blanks: &[("xlsx", include_bytes!("../../../packaging/windows/new/new.xlsx"))],
        exports: &["xlsx", "ods", "pdf", "csv", "tsv"],
    })
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
                self.write_in(
                    id,
                    move |worker, doc| worker.run(doc, "cell.set", &[&address, &text]),
                    move |_, done, _, _| ask.answer(done.map(|()| String::new()).map_err(failed)),
                    cx,
                );
            }
            [verb, _, target] if verb == "export" => self.export_for(id, PathBuf::from(target), ask, cx),
            _ => ask.answer(Err("unknown command".into())),
        }
    }

    /// Copies to `target`; CSV and TSV take the shown sheet.
    fn export_for(&mut self, id: u64, target: PathBuf, ask: Ask, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |book, _, cx| book.export_for(id, target, ask, cx));
        }
        let Some(work) = self.tab(id).map(|tab| tab.work.clone()) else {
            return ask.answer(Err(format!("tab {id} closed")));
        };
        let format = ely_office::files::save_as_format(&Self::KIND, &target);
        let ext = target.extension().map(|ext| ext.to_string_lossy().to_lowercase()).unwrap_or_default();
        let named = remote::path_json(&target.display().to_string());
        self.read_in(
            id,
            move |worker, doc| {
                remote::export_to(&work, &target, |copy| {
                    let url = ely_lok::file_url(copy);
                    match (format, ext.as_str()) {
                        (Some(format), _) => doc.save_as(worker, &url, format),
                        (None, "csv") => doc.export_text(worker, &url, ','),
                        (None, "tsv") => doc.export_text(worker, &url, '\t'),
                        (None, _) => doc.export_pdf(worker, &url, &crate::output::WHOLE),
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
        let sent = super::request(&["get", "a.xlsx", "S!A1"]).expect("a request");
        assert_eq!((sent[0].as_str(), sent[2].as_str()), ("get", "S!A1"));
        assert!(std::path::Path::new(&sent[1]).is_absolute(), "{sent:?}");
        assert!(super::request(&["get", "a.xlsx"]).is_err(), "too few words");
        assert!(super::request(&["drop", "a.xlsx"]).is_err(), "no such verb");
    }
}
