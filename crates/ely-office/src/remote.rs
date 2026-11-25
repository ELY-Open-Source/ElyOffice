//! Other programs read and write open documents (GitHub #19).

use std::path::{Path, PathBuf};

use ely_ui::Ask;
use gpui::{App, Context, TaskExt as _, Window};

use crate::{Core, Engine, Host, Opening as _, Saving, Tabs, in_window};

mod client;
pub mod grid;
pub mod length;
pub mod pairs;
mod text;
pub use client::{Cli, command_line};
pub use text::{body_paragraphs, body_text, slide_text};

/// `file` made absolute, as the running copy reads it.
pub fn absolute(file: &str) -> Result<String, String> {
    std::path::absolute(file).map(|file| file.display().to_string()).map_err(|error| format!("{file}: {error}"))
}

/// Files as a JSON array; paths may hold newlines.
pub fn files_json(files: &[String]) -> String {
    serde_json::Value::from(files).to_string()
}

/// One path as a JSON string.
pub fn path_json(path: &str) -> String {
    serde_json::Value::from(path).to_string()
}

/// Why an export onto an open file is refused.
const EXPORT_OPEN: &str = "{target}: cannot export over an open file; close it first";

/// `write` fills a copy in `work`, then moved to `target`.
pub fn export_to(work: &Path, target: &Path, write: impl FnOnce(&Path) -> std::io::Result<()>) -> std::io::Result<()> {
    let ext = target.extension().map(|ext| ext.to_string_lossy().to_lowercase()).unwrap_or_default();
    let copy = work.join(format!("export.{ext}"));
    write(&copy)?;
    std::fs::copy(&copy, target)?;
    std::fs::remove_file(&copy)
}

/// A copy of `doc` as `format` via `path`; `doc` stays.
pub fn copy<G: Engine>(worker: &mut G::Worker, doc: &G::Doc, path: &Path, format: &str) -> std::io::Result<Vec<u8>> {
    G::snapshot(worker, doc, path, format)?;
    let bytes = std::fs::read(path)?;
    std::fs::remove_file(path)?;
    Ok(bytes)
}

/// Hands `ask` to the window that holds its file (W.1b).
pub fn route<H: Remote>(ask: Ask, cx: &mut App, answer: fn(&mut H, Ask, &mut Context<H>)) {
    let hosts = match crate::shared::Handle::<H>::joined(cx) {
        Ok(shared) => shared.hosts(),
        Err(error) => return ask.answer(Err(error.to_string())),
    };
    // A malformed request is told so by the host.
    let words: Vec<String> = serde_json::from_str(&ask.request).unwrap_or_default();
    if words.len() == 1 && words[0] == "docs" {
        let files: Vec<String> = hosts.iter().flat_map(|host| host.read(cx).docs()).collect();
        return ask.answer(Ok(files_json(&files)));
    }
    let holder = words.get(1).and_then(|file| hosts.iter().find(|host| host.read(cx).tab_of(file).is_ok()));
    let Some(host) = holder.cloned().or_else(|| crate::shared::Handle::<H>::joined(cx).ok()?.front(None, cx)) else {
        return ask.answer(Err(format!("{} has no window open", H::KIND.app.name())));
    };
    host.update(cx, |host, cx| answer(host, ask, cx));
}

/// Answers `save`; other verbs come back.
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
            ask.answer(Ok(files_json(&self.docs())));
            return None;
        }
        // A hand-over may still be queued: opened here.
        if words.len() == 2 && words[0] == "open" && self.tab_of(&words[1]).is_err() {
            self.open_file(PathBuf::from(&words[1]), cx);
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
        // Loading, or waiting to be shown: asked again once loaded.
        if self.tab(id).is_none() {
            log::info!("{slug}: tab {id} loads for another program first");
            self.core_mut().wake(id, ask);
            cx.notify();
            return None;
        }
        if words.len() == 2 && words[0] == "open" {
            ask.answer(Ok(path_json(&words[1])));
            return None;
        }
        if words.len() == 3 && words[0] == "export" {
            return match self.holder(Path::new(&words[2])) {
                Ok(None) => Some((id, words, ask)),
                Ok(Some(_)) => {
                    ask.answer(Err(EXPORT_OPEN.replace("{target}", &words[2])));
                    None
                }
                Err(error) => {
                    ask.answer(Err(format!("{}: {error}", words[2])));
                    None
                }
            };
        }
        if !(words.len() == 2 && words[0] == "save") {
            return Some((id, words, ask));
        }
        if self.tab(id).is_some_and(|tab| tab.risk.is_some()) {
            ask.answer(Err(format!("{} must be saved under another name in {}", words[1], Self::KIND.app.name())));
            return None;
        }
        let busy = self.hold(id);
        cx.spawn(async move |this, cx| {
            let _busy = busy;
            in_window(&this, cx, |host, window, cx| host.save_for(id, ask, window, cx))
        })
        .detach_and_log_err(cx);
        None
    }

    /// This window's files, open or waiting.
    fn docs(&self) -> Vec<String> {
        let open = self.core().tabs.iter().filter_map(|tab| tab.session.path()).map(Path::new);
        open.chain(self.core().parked_files()).map(|path| path.display().to_string()).collect()
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
        let loading = |id: &u64| self.core().loading.iter().any(|(of, ..)| of == id);
        held.filter(|id| self.tab(*id).is_some() || loading(id))
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
            ask.answer(saved.clone().map(|path| path_json(&path.display().to_string())));
        }
    }
}
