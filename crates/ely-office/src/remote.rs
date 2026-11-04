//! Other programs read and write open documents (GitHub #19).

use std::{io::Write as _, path::Path};

use ely_platform::instance;
use ely_ui::Ask;
use gpui::{Context, TaskExt as _, Window};

use crate::{Core, Engine, Host, Saving, Tabs, in_window};

mod text;
pub use text::{body_text, slide_text};

/// Asks the running copy; `exact` verbs print as is.
pub fn command_line(
    app: ely_platform::AppId,
    noun: &str,
    usage: &str,
    request: fn(&[&str]) -> Result<Vec<String>, String>,
    exact: &[&str],
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
    let content = request.first().is_some_and(|verb| exact.contains(&verb.as_str()));
    match instance::ask(app, &serde_json::Value::from(request).to_string()) {
        Ok(answer) => {
            let line = match content || answer.is_empty() {
                true => answer,
                false => format!("{answer}
"),
            };
            // Exit runs no destructors: flush here.
            let mut out = std::io::stdout();
            match out.write_all(line.as_bytes()).and_then(|()| out.flush()) {
                Ok(()) => Some(0),
                Err(error) => {
                    eprintln!("{}: the answer did not print: {error}", app.slug());
                    Some(1)
                }
            }
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
        // A tab whose file went keeps its own name.
        let named = self.core().tabs.iter().find(|tab| tab.session.path().is_some_and(|path| Path::new(path) == Path::new(file)));
        if let Some(tab) = named {
            return Ok(tab.id);
        }
        // Matched in memory; only `file` is read.
        let held = self.holder(Path::new(file)).map_err(|error| format!("{file}: {error}"))?;
        held.filter(|id| self.tab(*id).is_some())
            .ok_or_else(|| format!("{file} is not open in {}", Self::KIND.app.name()))
    }

    /// Saves at the next pause; asks then share it.
    fn save_for(&mut self, id: u64, ask: Ask, window: &mut Window, cx: &mut Context<Self>) {
        let waiting = self.core().asked.iter().any(|(tab, _)| *tab == id);
        self.core_mut().asked.push((id, ask));
        if waiting {
            return log::info!("{}: tab {id}'s program save joins the one waiting", Self::KIND.app.slug());
        }
        crate::save::pause::at_pause(self, id, Box::new(move |host: &mut Self, window: &mut Window, cx: &mut Context<Self>| host.save_asked(id, window, cx)), window, cx);
    }

    /// The waiting asks of tab `id` get their own save.
    fn save_asked(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |host, window, cx| host.save_asked(id, window, cx));
        }
        if self.exporting(id) {
            return self.after_export(id, move |host, window, cx| host.save_asked(id, window, cx));
        }
        // A closed tab answered its asks already.
        if self.tab(id).is_none() {
            return;
        }
        let (asks, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut self.core_mut().asked).into_iter().partition(|(tab, _)| *tab == id);
        self.core_mut().asked = rest;
        // Only this save's end answers them.
        self.core_mut().saving.extend(asks);
        self.save_paused(id, None, window, cx);
    }
}

impl<H: Host + Saving> Remote for H {}

impl<H: Host> Core<H> {
    /// Answers every program whose save of tab `id` ended.
    pub(crate) fn answer_saves(&mut self, id: u64, saved: Result<&Path, String>) {
        // Each asked while no export ran: this one began after.
        let (ended, rest) = std::mem::take(&mut self.saving).into_iter().partition(|(tab, _)| *tab == id);
        self.saving = rest;
        for (_, ask) in ended {
            ask.answer(saved.clone().map(|path| path.display().to_string()));
        }
    }
}
