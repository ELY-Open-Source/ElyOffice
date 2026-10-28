//! Cloning with progress and a stop (TREE-01).

use std::{io, path::PathBuf};

use ely_gpui_component::{forms::TextInput, primitives::Severity};
use futures::{
    StreamExt as _,
    channel::{mpsc, oneshot},
};
use gpui::{AppContext as _, Context, SharedString, Window};

use crate::{
    asks::Ask,
    running::Stop,
    tree::{Tree, text},
};

/// A clone on its way.
pub struct Cloning {
    pub into: PathBuf,
    pub said: SharedString,
    pub stop: Stop,
    /// Asked to stop; its failure is then expected.
    pub stopping: bool,
}

/// A URL's last part, less `.git`.
pub fn suggested(url: &str) -> Option<&str> {
    let last = url.trim_end_matches('/').rsplit(['/', ':', '\\']).next()?;
    Some(last.strip_suffix(".git").unwrap_or(last)).filter(|name| !name.is_empty())
}

impl Tree {
    pub fn ask_url(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.cloning.is_some() {
            let words = text(cx, "tree.clone.busy", &[]);
            return self.toast(words, Severity::Warning, cx);
        }
        self.ask = Some(Ask::Url(cx.new(|cx| TextInput::new(window, cx))));
        cx.notify();
    }

    /// Asks where the clone goes, beside the last repository opened.
    pub fn pick_clone_folder(&mut self, url: String, cx: &mut Context<Self>) {
        let near = self
            .recent
            .first()
            .and_then(|last| last.parent())
            .map(PathBuf::from)
            .or_else(std::env::home_dir);
        let Some(near) = near else {
            return self.failed("tree.error.clone", "no home folder", cx);
        };
        let picked = cx.prompt_for_new_path(&near, suggested(&url));
        cx.spawn(async move |this, cx| {
            let picked = picked
                .await
                .map_err(|error| error.to_string())
                .and_then(|picked| picked.map_err(|error| error.to_string()));
            this.update(cx, |tree, cx| match picked {
                Ok(Some(into)) => tree.start_clone(url, into, cx),
                Ok(None) => log::info!("tree: the clone folder panel was cancelled"),
                Err(error) => tree.failed("tree.error.panel", &error, cx),
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn start_clone(&mut self, url: String, into: PathBuf, cx: &mut Context<Self>) {
        let Some(git) = self.git(cx) else { return };
        let running = match git.clone_repo(&url, &into) {
            Ok(running) => running,
            Err(error) => return self.failed("tree.error.clone", &error.to_string(), cx),
        };
        self.cloning = Some(Cloning {
            into,
            said: SharedString::default(),
            stop: running.stop(),
            stopping: false,
        });
        let (said, mut lines) = mpsc::unbounded();
        let (done, result) = oneshot::channel();
        // Git's progress blocks a thread until the clone ends.
        std::thread::spawn(move || {
            let followed = running.follow(|line| {
                let _ = said.unbounded_send(SharedString::from(line.to_string()));
            });
            let _ = done.send(followed);
        });
        cx.spawn(async move |this, cx| {
            while let Some(line) = lines.next().await {
                let shown = this.update(cx, |tree, cx| {
                    if let Some(cloning) = &mut tree.cloning {
                        cloning.said = line;
                        cx.notify();
                    }
                });
                if shown.is_err() {
                    return;
                }
            }
            let ended = result
                .await
                .unwrap_or_else(|_| Err(io::Error::other("the clone thread ended without a word")));
            this.update(cx, |tree, cx| tree.cloned(ended.map(drop), cx))
                .ok();
        })
        .detach();
        cx.notify();
    }

    fn cloned(&mut self, ended: io::Result<()>, cx: &mut Context<Self>) {
        let Some(cloning) = self.cloning.take() else {
            return;
        };
        cx.notify();
        match ended {
            Ok(()) => self.open_path(cloning.into, cx),
            Err(_) if cloning.stopping => {
                let words = text(cx, "tree.clone.stopped", &[]);
                self.toast(words, Severity::Info, cx);
            }
            Err(error) => self.failed("tree.error.clone", &error.to_string(), cx),
        }
    }

    /// Stops the clone; Git removes what it began.
    pub fn stop_clone(&mut self, cx: &mut Context<Self>) {
        let Some(cloning) = &mut self.cloning else {
            return;
        };
        // Only a stop asked for turns failure quiet.
        match cloning.stop.now() {
            Ok(()) => cloning.stopping = true,
            Err(error) => self.failed("tree.error.stop", &error.to_string(), cx),
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_url_suggests_its_last_part() {
        assert_eq!(
            super::suggested("https://github.com/a/ely.git"),
            Some("ely")
        );
        assert_eq!(super::suggested("git@github.com:a/ely.git/"), Some("ely"));
        assert_eq!(super::suggested("/srv/repos/notes"), Some("notes"));
        assert_eq!(super::suggested("https://host/.git"), None);
    }
}
