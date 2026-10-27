//! Other programs read and write open documents (GitHub #19).

use std::path::Path;

use ely_platform::instance;
use ely_ui::Ask;
use gpui::{Context, TaskExt as _, Window};

use crate::{Core, Engine, Host, Saving, Tabs, in_window};

mod text;
pub use text::{body_text, slide_text};

/// `<app> <noun> ...` asks the running copy; its exit code.
pub fn command_line(
    app: ely_platform::AppId,
    noun: &str,
    usage: &str,
    request: fn(&[&str]) -> Result<Vec<String>, String>,
) -> Option<i32> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_none_or(|first| first != noun) {
        return None;
    }
    ely_platform::startup::attach_console();
    let words: Result<Vec<&str>, _> = args[1..].iter().map(|arg| arg.to_str().ok_or("an argument is not text")).collect();
    let request = match words.map_err(String::from).and_then(|words| request(&words)) {
        Ok(request) => request,
        Err(why) => {
            eprintln!("{}: {why}\n{usage}", app.slug());
            return Some(2);
        }
    };
    match instance::ask(app, &serde_json::Value::from(request).to_string()) {
        Ok(answer) => {
            if !answer.is_empty() {
                println!("{answer}");
            }
            Some(0)
        }
        Err(error) => {
            eprintln!("{}: {error}", app.slug());
            Some(1)
        }
    }
}

/// `file` made absolute, as the running copy reads it.
pub fn absolute(file: &str) -> Result<String, String> {
    std::path::absolute(file).map(|file| file.display().to_string()).map_err(|error| format!("{file}: {error}"))
}

/// A copy of `doc` as `format` via `path`; `doc` stays.
pub fn copy<G: Engine>(worker: &mut G::Worker, doc: &G::Doc, path: &Path, format: &str) -> std::io::Result<Vec<u8>> {
    G::snapshot(worker, doc, path, format)?;
    let bytes = std::fs::read(path)?;
    std::fs::remove_file(path)?;
    Ok(bytes)
}

/// Answers `docs` and `save`; other verbs come back.
pub trait Remote: Saving {
    /// The verb's tab and words; None once answered.
    fn asked(&mut self, ask: Ask, cx: &mut Context<Self>) -> Option<(u64, Vec<String>, Ask)> {
        let words = match serde_json::from_str::<Vec<String>>(&ask.request) {
            Ok(words) => words,
            Err(error) => {
                ask.answer(Err(format!("not a request: {error}")));
                return None;
            }
        };
        let slug = Self::KIND.app.slug();
        log::info!("{slug}: another program asks {}", words.first().map_or("nothing", String::as_str));
        if words.len() == 1 && words[0] == "docs" {
            let files: Vec<String> = self.core().tabs.iter().filter_map(|tab| tab.session.path()).map(|path| Path::new(path).display().to_string()).collect();
            ask.answer(Ok(files.join("\n")));
            return None;
        }
        let id = match words.get(1).map(|file| self.tab_of(file)) {
            Some(Ok(id)) => id,
            Some(Err(why)) => {
                ask.answer(Err(why));
                return None;
            }
            None => {
                ask.answer(Err("unknown command".into()));
                return None;
            }
        };
        if !(words.len() == 2 && words[0] == "save") {
            return Some((id, words, ask));
        }
        if self.tab(id).is_some_and(|tab| tab.risk.is_some()) {
            ask.answer(Err(format!("{} must be saved under another name in {}", words[1], Self::KIND.app.name())));
            return None;
        }
        cx.spawn(async move |this, cx| in_window(&this, cx, |host, window, cx| host.save_for(id, ask, window, cx)))
            .detach_and_log_err(cx);
        None
    }

    /// The tab showing `file`; only saved files are named.
    fn tab_of(&self, file: &str) -> Result<u64, String> {
        let wanted = std::fs::canonicalize(file).map_err(|error| format!("{file}: {error}"))?;
        // A tab whose file went cannot match.
        let shows = |path: &Path| std::fs::canonicalize(path).is_ok_and(|real| real == wanted);
        let tab = self.core().tabs.iter().find(|tab| tab.session.path().is_some_and(|path| shows(Path::new(path))));
        tab.map(|tab| tab.id).ok_or_else(|| format!("{file} is not open in {}", Self::KIND.app.name()))
    }

    /// Saves once idle; the next save ending is this one.
    fn save_for(&mut self, id: u64, ask: Ask, window: &mut Window, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |host, window, cx| host.save_for(id, ask, window, cx));
        }
        if self.exporting(id) {
            return self.after_export(id, move |host, window, cx| host.save_for(id, ask, window, cx));
        }
        if self.tab(id).is_none() {
            return ask.answer(Err(format!("tab {id} closed before its save")));
        }
        self.core_mut().saving.push((id, ask));
        self.save(id, None, window, cx);
    }
}

impl<H: Host + Saving> Remote for H {}

impl<H: Host> Core<H> {
    /// Answers the program whose save of tab `id` ended.
    pub(crate) fn answer_saves(&mut self, id: u64, saved: Result<&Path, String>) {
        if let Some(at) = self.saving.iter().position(|(tab, _)| *tab == id) {
            self.saving.remove(at).1.answer(saved.map(|path| path.display().to_string()));
        }
    }
}
