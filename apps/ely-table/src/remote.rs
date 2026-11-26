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

mod format;
mod range;
mod structure;

const USAGE: &str = "usage: ely-table docs | open FILE | new FILE | export FILE TARGET | save FILE
       ely-table cell get FILE ADDRESS | cell set FILE ADDRESS TEXT
       ely-table range get FILE ADDRESS | range set FILE ADDRESS DATA
       ely-table range merge|unmerge FILE ADDRESS
       ely-table format FILE ADDRESS PROPERTY=VALUE ...
       ely-table sheet list FILE
       ely-table sheet add FILE NAME [POSITION]
       ely-table sheet remove FILE SHEET
       ely-table sheet rename FILE SHEET NAME
       ely-table sheet freeze FILE ADDRESS
       ely-table rows insert|delete FILE ADDRESS
       ely-table rows height FILE ADDRESS SIZE
       ely-table columns insert|delete FILE ADDRESS
       ely-table columns width FILE ADDRESS SIZE
open and new start ElyTable if needed; the rest require it running (with FILE open).
ADDRESS names its sheet (Sheet1!A1, Sheet1!5:7, Sheet1!C:E); each write is one undo step.
cell get prints JSON; cell set writes TEXT (= for a formula).
range: bare sheet = used area; DATA is a JSON 2-D array or TSV (- for stdin); \"\" clears a cell.
range merge keeps hidden cell values for unmerge.
format sets PROPERTY=VALUE ... in one undo step: bold, italic, underline, wrap, font, size, color,
background, number, align, valign, border, border-left|top|right|bottom|inner-h|inner-v.
border: all|outer|none; border-*: none or style:width:#RRGGBB (width pt/mm, max 2mm).
sheet: list prints JSON; POSITION starts at 1; freeze locks above/left of ADDRESS (A1 unfreezes).
SIZE is a number with cm, mm, in or pt (e.g. 2.5cm), or auto.
Any argument written as - is read from standard input, up to 64 MiB.
Exit codes: 0 success, 1 failed, 2 wrong usage, 3 not running.";

/// `ely-table open|new|export|cell|range|format|sheet|rows|columns ...`; its exit code.
pub fn command_line() -> Option<i32> {
    remote::command_line(&Cli {
        app: AppId::Table,
        nouns: &[
            ("cell", request),
            ("range", range::request),
            ("format", format::request),
            ("sheet", structure::sheet),
            ("rows", structure::rows),
            ("columns", structure::columns),
        ],
        usage: USAGE,
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
        let verb = words.first().map_or("", String::as_str);
        if ["sheet.", "rows.", "columns."].iter().any(|noun| verb.starts_with(noun)) || ["range.merge", "range.unmerge", "range.format"].contains(&verb) {
            return self.answer_structure(id, words, ask, cx);
        }
        if verb.starts_with("range.") {
            return self.answer_range(id, words, ask, cx);
        }
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
            // A waiting file has loaded by now.
            [verb, _] if verb == "score.load" => ask.answer(Ok(String::new())),
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
