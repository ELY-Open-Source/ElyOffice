//! Other programs read and write open workbooks (GitHub #19).

use std::path::Path;

use ely_lok::sheet;
use ely_office::prelude::*;
use ely_platform::{AppId, instance};
use ely_ui::Ask;
use gpui::{Context, TaskExt as _, Window};

use crate::book::Book;

const USAGE: &str = "usage: ely-table cell docs
       ely-table cell get FILE ADDRESS
       ely-table cell set FILE ADDRESS TEXT
       ely-table cell save FILE
ADDRESS names its sheet, as in Sheet1!A1. ElyTable must be running with FILE open.";

/// `ely-table cell ...` asks the running copy; its exit code.
pub fn command_line() -> Option<i32> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_none_or(|first| first != "cell") {
        return None;
    }
    ely_platform::startup::attach_console();
    let request = match request(&args[1..]) {
        Ok(request) => request,
        Err(why) => {
            eprintln!("ely-table: {why}\n{USAGE}");
            return Some(2);
        }
    };
    match instance::ask(AppId::Table, &serde_json::Value::from(request).to_string()) {
        Ok(answer) => {
            if !answer.is_empty() {
                println!("{answer}");
            }
            Some(0)
        }
        Err(error) => {
            eprintln!("ely-table: {error}");
            Some(1)
        }
    }
}

/// The words sent; the file made absolute.
fn request(args: &[std::ffi::OsString]) -> Result<Vec<String>, String> {
    let words: Vec<&str> = args.iter().map(|arg| arg.to_str().ok_or("an argument is not text")).collect::<Result<_, _>>()?;
    let file = |file: &str| std::path::absolute(file).map(|file| file.display().to_string()).map_err(|error| format!("{file}: {error}"));
    Ok(match words[..] {
        ["docs"] => vec!["docs".into()],
        ["get", path, address] => vec!["get".into(), file(path)?, address.into()],
        ["set", path, address, text] => vec!["set".into(), file(path)?, address.into(), text.into()],
        ["save", path] => vec!["save".into(), file(path)?],
        _ => return Err("unknown command".into()),
    })
}

impl Book {
    /// Answers a request; the kernel works off the window thread.
    pub fn answer(&mut self, ask: Ask, cx: &mut Context<Self>) {
        let words = match serde_json::from_str::<Vec<String>>(&ask.request) {
            Ok(words) => words,
            Err(error) => return ask.answer(Err(format!("not a request: {error}"))),
        };
        log::info!("table: another program asks {}", words.first().map_or("nothing", String::as_str));
        if words.len() == 1 && words[0] == "docs" {
            let files: Vec<String> = self.core.tabs.iter().filter_map(|tab| tab.session.path()).map(|path| Path::new(path).display().to_string()).collect();
            return ask.answer(Ok(files.join("\n")));
        }
        let Some(id) = words.get(1).map(|file| self.tab_of(file)) else {
            return ask.answer(Err("unknown command".into()));
        };
        let id = match id {
            Ok(id) => id,
            Err(why) => return ask.answer(Err(why)),
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
            [verb, file] if verb == "save" => {
                if self.tab(id).is_some_and(|tab| tab.risk.is_some()) {
                    return ask.answer(Err(format!("{file} must be saved under another name in ElyTable")));
                }
                cx.spawn(async move |this, cx| ely_office::in_window(&this, cx, |book, window, cx| book.save_for(id, ask, window, cx)))
                    .detach_and_log_err(cx);
            }
            _ => ask.answer(Err("unknown command".into())),
        }
    }

    /// The tab showing `file`; only saved files are named.
    fn tab_of(&self, file: &str) -> Result<u64, String> {
        let wanted = std::fs::canonicalize(file).map_err(|error| format!("{file}: {error}"))?;
        // A tab whose file went cannot match.
        let shows = |path: &Path| std::fs::canonicalize(path).is_ok_and(|real| real == wanted);
        let tab = self.core.tabs.iter().find(|tab| tab.session.path().is_some_and(|path| shows(Path::new(path))));
        tab.map(|tab| tab.id).ok_or_else(|| format!("{file} is not open in ElyTable"))
    }

    /// Saves once idle; the next save ending is this one.
    fn save_for(&mut self, id: u64, ask: Ask, window: &mut Window, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |book, window, cx| book.save_for(id, ask, window, cx));
        }
        if self.exporting(id) {
            return self.after_export(id, move |book, window, cx| book.save_for(id, ask, window, cx));
        }
        if self.tab(id).is_none() {
            return ask.answer(Err(format!("tab {id} closed before its save")));
        }
        self.saving.push((id, ask));
        self.save(id, None, window, cx);
    }

    /// Answers the program whose save of tab `id` ended.
    pub(crate) fn answer_saves(&mut self, id: u64, saved: Result<&Path, String>) {
        if let Some(at) = self.saving.iter().position(|(tab, _)| *tab == id) {
            self.saving.remove(at).1.answer(saved.map(|path| path.display().to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_words_carry_the_file_made_absolute() {
        let args = |words: &[&str]| words.iter().map(std::ffi::OsString::from).collect::<Vec<_>>();
        let sent = super::request(&args(&["get", "a.xlsx", "S!A1"])).expect("a request");
        assert_eq!((sent[0].as_str(), sent[2].as_str()), ("get", "S!A1"));
        assert!(std::path::Path::new(&sent[1]).is_absolute(), "{sent:?}");
        assert!(super::request(&args(&["get", "a.xlsx"])).is_err(), "too few words");
        assert!(super::request(&args(&["drop", "a.xlsx"])).is_err(), "no such verb");
    }
}
