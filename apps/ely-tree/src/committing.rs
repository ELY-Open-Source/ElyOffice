//! The commit box: message, amend, identity (TREE-05).

use ely_gpui_component::forms::TextInput;
use gpui::{AppContext as _, Context, Window};

use crate::{
    asks::Ask,
    commit::{self, Me},
    tree::{Tree, text},
    work::Write,
};

/// A repository's commit box, kept while another is shown.
#[derive(Clone, Debug, Default)]
pub struct Held {
    pub subject: String,
    pub body: String,
    pub amend: bool,
    pub draft: Option<(String, String)>,
}

impl Tree {
    /// Shows repository `id`, swapping its commit box in.
    pub fn activate(&mut self, id: Option<u64>, cx: &mut Context<Self>) {
        if self.active == id {
            return cx.notify();
        }
        // A last message still loading is not carried across.
        if self.amend_loading {
            self.set_amend(false, cx);
        }
        let held = Held {
            subject: self.subject.read(cx).text().to_string(),
            body: self.body.read(cx).text().to_string(),
            amend: self.amend,
            draft: self.draft.take(),
        };
        if let Some(old) = self.active.and_then(|old| self.opened_mut(old)) {
            old.held = held;
        }
        self.active = id;
        self.amend_ask += 1;
        let held = id
            .and_then(|id| self.opened_mut(id))
            .map(|opened| std::mem::take(&mut opened.held))
            .unwrap_or_default();
        (self.amend, self.draft) = (held.amend, held.draft);
        self.fill(&held.subject, &held.body, cx);
        self.ensure_side(cx);
        cx.notify();
    }

    /// Rereads who commits and whether Git signs.
    pub fn read_me(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(git) = self.git.as_ref().ok().cloned() else {
            return;
        };
        let Some(opened) = self.opened_mut(id) else {
            return;
        };
        opened.me_asks += 1;
        let (root, ask) = (opened.repo.root.clone(), opened.me_asks);
        let task = cx.background_spawn(async move { commit::me(&git, &root) });
        cx.spawn(async move |this, cx| {
            let me = task.await.map_err(|error| error.to_string());
            this.update(cx, |tree, cx| {
                if let Some(opened) = tree.opened_mut(id).filter(|opened| opened.me_asks == ask) {
                    opened.me = Some(me);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn fill(&self, subject: &str, body: &str, cx: &mut Context<Self>) {
        self.subject
            .update(cx, |field, cx| field.set_text(subject.to_string(), cx));
        self.body
            .update(cx, |field, cx| field.set_text(body.to_string(), cx));
    }

    /// Amend loads the last message; off restores the draft.
    pub fn set_amend(&mut self, on: bool, cx: &mut Context<Self>) {
        // Any later choice voids a read still on its way.
        self.amend_ask += 1;
        if !on {
            (self.amend, self.amend_loading) = (false, false);
            if let Some((subject, body)) = self.draft.take() {
                self.fill(&subject, &body, cx);
            }
            return cx.notify();
        }
        let Some(git) = self.git(cx) else { return };
        let Some((repo, root)) = self
            .active()
            .map(|opened| (opened.id, opened.repo.root.clone()))
        else {
            return;
        };
        // The tick shows at once; the message follows.
        (self.amend, self.amend_loading) = (true, true);
        self.draft = Some((
            self.subject.read(cx).text().to_string(),
            self.body.read(cx).text().to_string(),
        ));
        let ask = self.amend_ask;
        let task = cx.background_spawn(async move { commit::last_message(&git, &root) });
        cx.spawn(async move |this, cx| {
            let last = task.await;
            this.update(cx, |tree, cx| {
                if tree.amend_ask != ask || tree.active != Some(repo) {
                    return log::info!("tree: an amend read came back late");
                }
                match last {
                    Ok(message) => {
                        let (subject, body) = commit::split(&message);
                        tree.fill(&subject, &body, cx);
                        tree.amend_loading = false;
                        cx.notify();
                    }
                    Err(error) => {
                        tree.set_amend(false, cx);
                        tree.failed("tree.error.amend", &error, cx);
                    }
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub fn commit(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.active.filter(|_| !self.amend_loading) else {
            return;
        };
        let message = commit::message(self.subject.read(cx).text(), self.body.read(cx).text());
        let amend = self.amend;
        log::info!("tree: commit asked, amend {amend}");
        self.write(id, Write::Commit { message, amend }, cx);
    }

    /// A commit went in: its box empties if unchanged since.
    pub fn committed(&mut self, id: u64, message: &str, cx: &mut Context<Self>) {
        if self.active == Some(id) {
            let now = commit::message(self.subject.read(cx).text(), self.body.read(cx).text());
            if now == message {
                (self.amend, self.draft) = (false, None);
                self.fill("", "", cx);
            }
        } else if let Some(opened) = self.opened_mut(id)
            && commit::message(&opened.held.subject, &opened.held.body) == message {
                opened.held = Held::default();
            }
        cx.notify();
    }

    /// Opens the identity dialog, filled with Git's settings.
    pub fn ask_identity(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let me: Me = match self.active().and_then(|opened| opened.me.clone()) {
            Some(Ok(me)) => me,
            _ => Me::default(),
        };
        let field =
            |value: Option<String>, key: &str, window: &mut Window, cx: &mut Context<Self>| {
                // Named for assistive tech; the dialog draws the label.
                let label = text(cx, key, &[]);
                cx.new(|cx| {
                    let mut field = TextInput::new(window, cx).label(label);
                    field.set_text(value.unwrap_or_default(), cx);
                    field
                })
            };
        let name = field(me.name, "tree.identity.name", window, cx);
        let email = field(me.email, "tree.identity.email", window, cx);
        let Some(repo) = self.active else { return };
        self.ask = Some(Ask::Identity {
            repo,
            name,
            email,
            global: false,
        });
        cx.notify();
    }

    /// Writes the name and address, here or for every repository.
    pub fn set_identity(&mut self, cx: &mut Context<Self>) {
        let Some(Ask::Identity {
            repo,
            name,
            email,
            global,
        }) = &self.ask
        else {
            return;
        };
        let id = *repo;
        let (name, email, global) = (
            name.read(cx).text().trim().to_string(),
            email.read(cx).text().trim().to_string(),
            *global,
        );
        if name.is_empty() || email.is_empty() {
            let words = text(cx, "tree.identity.empty", &[]);
            return self.toast(words, ely_gpui_component::primitives::Severity::Warning, cx);
        }
        self.ask = None;
        for (key, value) in [("user.name", name), ("user.email", email)] {
            self.write(
                id,
                Write::Config {
                    key: key.into(),
                    value,
                    global,
                },
                cx,
            );
        }
        cx.notify();
    }
}
