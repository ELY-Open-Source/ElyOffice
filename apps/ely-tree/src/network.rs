//! Fetch, pull and push, with progress and a stop (TREE-11).

use std::io;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    feedback::Alert,
    primitives::Severity,
    theme::ControlSize,
};
use futures::{
    StreamExt as _,
    channel::{mpsc, oneshot},
};
use gpui::{AnyElement, AppContext as _, Context, IntoElement, SharedString};

use crate::{
    asks::{Ask, Choice},
    ops, remotes,
    running::Stop,
    tree::{Tree, text},
    work::Write,
};

/// A remote run under way in one repository.
pub struct Net {
    pub verb: String,
    pub said: SharedString,
    pub stop: Stop,
    /// Asked to stop; its failure is then expected.
    pub stopping: bool,
}

fn args(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| word.to_string()).collect()
}

impl Tree {
    /// Runs a queued remote write, line by line.
    pub(crate) fn start_remote(&mut self, id: u64, args: Vec<String>, cx: &mut Context<Self>) {
        let Some(git) = self.git.as_ref().ok().cloned() else {
            return;
        };
        let Some(opened) = self.opened_mut(id) else {
            return;
        };
        let root = opened.repo.root.clone();
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        let running = match git.start(&root, &words) {
            Ok(running) => running,
            Err(error) => return self.written(id, &Write::Remote(args), Err(error), false, cx),
        };
        opened.net = Some(Net {
            verb: args.first().cloned().unwrap_or_default(),
            said: SharedString::default(),
            stop: running.stop(),
            stopping: false,
        });
        let (said, mut lines) = mpsc::unbounded();
        let (done, result) = oneshot::channel();
        let asked = args.clone();
        // Git's progress blocks a thread until it ends.
        std::thread::spawn(move || {
            let followed = running.follow(|line| {
                let _ = said.unbounded_send(SharedString::from(line.to_string()));
            });
            let stopped = match &followed {
                Err(error) => ops::stopped(&git, &root, &asked, &error.to_string()),
                Ok(_) => false,
            };
            let _ = done.send((followed.map(drop), stopped));
        });
        cx.spawn(async move |this, cx| {
            while let Some(line) = lines.next().await {
                let shown = this.update(cx, |tree, cx| {
                    if let Some(net) = tree.opened_mut(id).and_then(|o| o.net.as_mut()) {
                        net.said = line;
                        cx.notify();
                    }
                });
                if shown.is_err() {
                    return;
                }
            }
            let (ended, stopped) = result
                .await
                .unwrap_or_else(|_| (Err(io::Error::other("the run ended without a word")), false));
            this.update(cx, |tree, cx| {
                tree.remote_done(id, args, ended, stopped, cx)
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn remote_done(
        &mut self,
        id: u64,
        args: Vec<String>,
        ended: io::Result<()>,
        stopped: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(opened) = self.opened_mut(id) else {
            return;
        };
        let net = opened.net.take();
        if ended.is_err() && net.is_some_and(|net| net.stopping) {
            // Asked for; later writes counted on it.
            opened.writes.clear();
            let words = text(cx, "tree.net.stopped", &[]);
            self.toast(words, Severity::Info, cx);
            return self.written(id, &Write::Remote(args), Ok(()), false, cx);
        }
        self.written(id, &Write::Remote(args), ended, stopped, cx);
    }

    /// Stops this repository's remote run; Git cleans up.
    pub fn stop_remote(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(net) = self.opened_mut(id).and_then(|opened| opened.net.as_mut()) else {
            return;
        };
        // Only a stop asked for turns failure quiet.
        match net.stop.now() {
            Ok(()) => net.stopping = true,
            Err(error) => self.failed("tree.error.stop_remote", &error.to_string(), cx),
        }
        cx.notify();
    }

    /// Nothing runs on behind a closed window.
    pub fn stop_all(&mut self, cx: &mut Context<Self>) {
        self.stop_clone(cx);
        let running: Vec<u64> = self
            .repos
            .iter()
            .filter(|opened| opened.net.is_some())
            .map(|opened| opened.id)
            .collect();
        for id in running {
            self.stop_remote(id, cx);
        }
    }

    fn remote(&mut self, words: &[&str], cx: &mut Context<Self>) {
        let Some(id) = self.active else { return };
        self.write(id, Write::Remote(args(words)), cx);
    }

    pub fn fetch_all(&mut self, cx: &mut Context<Self>) {
        self.remote(&["fetch", "--progress", "--all"], cx);
    }

    pub fn fetch_one(&mut self, name: &str, cx: &mut Context<Self>) {
        self.remote(&["fetch", "--progress", "--", name], cx);
    }

    pub fn pull(&mut self, cx: &mut Context<Self>) {
        self.remote(&["pull", "--progress"], cx);
    }

    /// With no upstream, asks where it goes first.
    pub fn push(&mut self, cx: &mut Context<Self>) {
        let Some(git) = self.git.as_ref().ok().cloned() else {
            return;
        };
        let Some(opened) = self.active() else { return };
        let Some(Ok(status)) = &opened.status else {
            return;
        };
        let Some(branch) = status.head.branch.clone() else {
            let words = text(cx, "tree.push.detached", &[]);
            return self.toast(words, Severity::Warning, cx);
        };
        if status.head.upstream.is_some() {
            return self.remote(&["push", "--progress"], cx);
        }
        let (repo, root) = (opened.id, opened.repo.root.clone());
        let task = cx.background_spawn(async move { remotes::list(&git, &root) });
        cx.spawn(async move |this, cx| {
            let read = task.await;
            this.update(cx, |tree, cx| {
                if tree.active != Some(repo) {
                    return;
                }
                let ask = match read {
                    Ok(list) if list.is_empty() => {
                        let words = text(cx, "tree.push.no_remote", &[]);
                        return tree.toast(words, Severity::Warning, cx);
                    }
                    Ok(list) => Ask::Choose {
                        repo,
                        title: text(cx, "tree.push.where", &[("branch", &branch)]),
                        choices: list.into_iter().map(|remote| remote.name).collect(),
                        then: Choice::Publish(branch),
                    },
                    Err(error) => Ask::Failed {
                        title: text(cx, "tree.remotes.unread", &[]),
                        detail: error.to_string().into(),
                    },
                };
                tree.offer(ask, cx);
            })
            .ok();
        })
        .detach();
    }

    /// Pushes `branch` to `remote` and tracks it there.
    pub fn publish(&mut self, repo: u64, branch: &str, remote: &str, cx: &mut Context<Self>) {
        let target = format!("refs/heads/{branch}:refs/heads/{branch}");
        let words = [
            "push",
            "--progress",
            "--set-upstream",
            "--",
            remote,
            &target,
        ];
        self.write(repo, Write::Remote(args(&words)), cx);
    }

    /// Reads what a forced push would overwrite, then asks.
    pub fn ask_force(&mut self, cx: &mut Context<Self>) {
        let Some(git) = self.git.as_ref().ok().cloned() else {
            return;
        };
        let Some(opened) = self.active() else { return };
        let (repo, root) = (opened.id, opened.repo.root.clone());
        let task = cx.background_spawn(async move { remotes::force(&git, &root) });
        cx.spawn(async move |this, cx| {
            let plan = task.await;
            this.update(cx, |tree, cx| {
                if tree.active != Some(repo) {
                    return;
                }
                let ask = match plan {
                    Ok(plan) => Ask::Force { repo, plan },
                    Err(error) => Ask::Failed {
                        title: text(cx, "tree.force.unready", &[]),
                        detail: error.to_string().into(),
                    },
                };
                tree.offer(ask, cx);
            })
            .ok();
        })
        .detach();
    }

    /// The running remote command, and its stop.
    pub fn net_alert(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let opened = self.active()?;
        let net = opened.net.as_ref()?;
        let id = opened.id;
        let verb = text(cx, &format!("tree.net.running.{}", net.verb), &[]);
        let this = cx.weak_entity();
        let stop = Button::new("tree-net-stop", text(cx, "tree.net.stop", &[]))
            .size(ControlSize::Sm)
            .variant(ButtonVariant::Secondary)
            .disabled(net.stopping)
            .on_click(move |_, _, cx| {
                this.update(cx, |tree, cx| tree.stop_remote(id, cx)).ok();
            });
        let alert = Alert::new("tree-net", Severity::Info, verb).body(net.said.clone());
        if !crate::running::STOPS {
            return Some(alert.into_any_element());
        }
        Some(alert.action(stop.into_any_element()).into_any_element())
    }
}
