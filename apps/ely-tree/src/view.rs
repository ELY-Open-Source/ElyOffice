//! What the window draws.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    feedback::{EmptyState, ErrorView},
    lists::{List, ListItem},
    navigation::{EditorTab, EditorTabs},
    primitives::IconName,
    shell::{StatusBar, StatusBarItem},
};
use ely_platform::AppId;
use ely_ui::{Shell, commands};
use gpui::{
    AnyElement, App, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    SharedString, Styled as _, Window, div,
};

use crate::{
    menu,
    status::{Head, State},
    tree::{Side, Tree, text},
};

fn name(path: &std::path::Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// Where HEAD is, in words.
pub fn branch(head: &Head, cx: &App) -> SharedString {
    match (&head.branch, &head.oid) {
        (Some(branch), Some(_)) => branch.clone().into(),
        (Some(branch), None) => text(cx, "tree.branch.unborn", &[("branch", branch)]),
        (None, oid) => {
            let short = oid.as_deref().map_or("", |oid| &oid[..oid.len().min(7)]);
            text(cx, "tree.branch.detached", &[("oid", short)])
        }
    }
}

impl Tree {
    fn tab_strip(&self, cx: &mut Context<Self>) -> Option<EditorTabs> {
        if self.repos.is_empty() {
            return None;
        }
        let tabs = self
            .repos
            .iter()
            .map(|opened| EditorTab::new(opened.id.to_string(), name(&opened.repo.root)));
        let (select, close) = (cx.weak_entity(), cx.weak_entity());
        let strip = EditorTabs::new("tree-tabs", tabs.collect::<Vec<_>>())
            .on_select(move |id, _, cx| {
                let id = id.parse().expect("tab ids are numbers");
                select
                    .update(cx, |tree, cx| tree.activate(Some(id), cx))
                    .ok();
            })
            .on_close(move |id, _, cx| {
                let id = id.parse().expect("tab ids are numbers");
                close.update(cx, |tree, cx| tree.close(id, cx)).ok();
            });
        Some(match self.active {
            Some(id) => strip.selected(id.to_string()),
            None => strip,
        })
    }

    fn status_bar(&self, cx: &mut Context<Self>) -> StatusBar {
        let mut bar = StatusBar::new();
        if let Some(opened) = self.active() {
            if let Some(Ok(status)) = &opened.status {
                let head = &status.head;
                bar = bar.left(
                    StatusBarItem::new("tree-branch")
                        .icon(IconName::GitBranch)
                        .label(branch(head, cx)),
                );
                if let Some(upstream) = &head.upstream {
                    let words = match head.counts {
                        Some((ahead, behind)) => text(
                            cx,
                            "tree.ahead_behind",
                            &[
                                ("upstream", upstream),
                                ("ahead", &ahead.to_string()),
                                ("behind", &behind.to_string()),
                            ],
                        ),
                        None => text(cx, "tree.upstream_gone", &[("upstream", upstream)]),
                    };
                    bar = bar.left(StatusBarItem::new("tree-upstream").label(words));
                }
            }
            bar = bar.left(
                StatusBarItem::new("tree-path").label(opened.repo.root.display().to_string()),
            );
        }
        bar
    }

    fn start_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let this = cx.weak_entity();
        let button = |id: &'static str, key: &str, run: fn(&mut Window, &mut App)| {
            Button::new(id, text(cx, key, &[])).on_click(move |_, window, cx| run(window, cx))
        };
        let empty = EmptyState::new(
            "tree-start",
            IconName::GitBranch,
            text(cx, "tree.empty", &[]),
        )
        .action(
            button("tree-open", "tree.open", |window, cx| {
                window.dispatch_action(Box::new(commands::Open), cx)
            })
            .variant(ButtonVariant::Primary),
        )
        .action(button("tree-new", "tree.new", |window, cx| {
            window.dispatch_action(Box::new(commands::New), cx)
        }))
        .action(button("tree-clone", "tree.clone", |window, cx| {
            window.dispatch_action(Box::new(menu::CloneRepository), cx)
        }));
        let recent = self.recent.iter().enumerate().map(|(at, path)| {
            let (this, path) = (this.clone(), path.clone());
            ListItem::new(("tree-recent", at), name(&path))
                .description(path.display().to_string())
                .on_click(move |_, _, cx| {
                    this.update(cx, |tree, cx| tree.open_path(path.clone(), cx))
                        .ok();
                })
                .into_any_element()
        });
        div()
            .flex()
            .flex_col()
            .items_center()
            .child(empty)
            .child(div().w_full().max_w_96().child(List::new().children(recent)))
            .into_any_element()
    }

    fn body(&self, cx: &mut Context<Self>) -> AnyElement {
        if let Err(error) = &self.git {
            return ErrorView::new(
                "tree-no-git",
                IconName::TriangleAlert,
                text(cx, "tree.no_git", &[]),
            )
            .detail(error.clone())
            .into_any_element();
        }
        if let Some(cloning) = &self.cloning {
            let this = cx.weak_entity();
            let title = text(cx, "tree.cloning", &[("name", &name(&cloning.into))]);
            let state = EmptyState::new("tree-cloning", IconName::Download, title)
                .body(cloning.said.clone());
            let stop = Button::new("tree-clone-stop", text(cx, "tree.clone.stop", &[]))
                .disabled(cloning.stopping)
                .on_click(move |_, _, cx| {
                    this.update(cx, |tree, cx| tree.stop_clone(cx)).ok();
                });
            return state.action(stop).into_any_element();
        }
        let Some(opened) = self.active() else {
            return self.start_page(cx);
        };
        if let Some(look) = self.look.as_ref().filter(|look| look.repo() == opened.id) {
            return self.look_pane(look, cx);
        }
        match self.side {
            Side::History => return self.commit_pane(&opened.hist, cx),
            Side::Branches => return self.refs_pane(cx),
            Side::Places => return self.places_pane(cx),
            Side::Changes => {}
        }
        if let Some(shown) = self.shown.as_ref().filter(|shown| shown.repo == opened.id) {
            return self.pane(shown, cx);
        }
        match &opened.status {
            None => div().into_any_element(),
            Some(Err(error)) => ErrorView::new(
                "tree-unread",
                IconName::TriangleAlert,
                text(cx, "tree.unreadable", &[]),
            )
            .detail(error.clone())
            .into_any_element(),
            Some(Ok(status)) => {
                let changed = status
                    .entries
                    .iter()
                    .filter(|entry| entry.state != State::Ignored)
                    .count();
                let (icon, title) = summary(changed, cx);
                EmptyState::new(("tree-summary", opened.id), icon, title).into_any_element()
            }
        }
    }
}

impl Render for Tree {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title: SharedString = self.active().map_or_else(
            || AppId::Tree.name().into(),
            |opened| name(&opened.repo.root).into(),
        );
        let left = self
            .active()
            .filter(|opened| matches!(opened.status, Some(Ok(_))))
            .map(|_| self.changes.clone().into());
        let mut shell = Shell::new(title, &self.focus)
            .left(left)
            .status(self.status_bar(cx))
            .children(self.net_alert(cx))
            .children(self.op_alert(cx))
            .child(self.body(cx));
        if let Some(strip) = self.tab_strip(cx) {
            shell = shell.tabs(strip);
        }
        div()
            .size_full()
            .key_context(menu::CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(Tree::on_open))
            .on_action(cx.listener(Tree::on_new))
            .on_action(cx.listener(Tree::on_clone))
            .on_action(cx.listener(Tree::on_close))
            .on_action(cx.listener(|tree, _: &menu::Fetch, _, cx| tree.fetch_all(cx)))
            .on_action(cx.listener(|tree, _: &menu::Pull, _, cx| tree.pull(cx)))
            .on_action(cx.listener(|tree, _: &menu::Push, _, cx| tree.push(cx)))
            .on_action(cx.listener(|tree, _: &menu::ForcePush, _, cx| tree.ask_force(cx)))
            .on_action(cx.listener(|tree, _: &menu::OpenTerminal, _, cx| tree.open_terminal(cx)))
            .on_action(cx.listener(|tree, _: &menu::Reveal, _, cx| tree.reveal(cx)))
            .on_action(cx.listener(|tree, _: &menu::CleanUntracked, _, cx| tree.ask_clean(cx)))
            .on_action(
                cx.listener(|tree, _: &menu::ChooseEditor, _, cx| tree.pick_editor(None, cx)),
            )
            .child(shell)
            .children(self.dialog(cx))
    }
}

/// The work tree's line: clean, or how many files changed.
pub(crate) fn summary(changed: usize, cx: &App) -> (IconName, SharedString) {
    match changed {
        0 => (IconName::CircleCheck, text(cx, "tree.clean", &[])),
        n => (IconName::GitBranch, ely_ui::i18n::counted(cx, "tree.changed", n as u64, &[("n", &n.to_string())])),
    }
}

#[cfg(test)]
mod tests {
    #[gpui::test]
    fn the_summary_counts_files_in_both_languages(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            ely_ui::i18n::install_as("en-US", cx);
            let said = |n: usize, cx: &gpui::App| super::summary(n, cx).1.to_string();
            assert_eq!([said(0, cx), said(1, cx), said(2, cx)], ["Nothing to commit", "1 file changed", "2 files changed"]);
            ely_ui::i18n::install_as("zh-CN", cx);
            assert_eq!([said(1, cx), said(2, cx)], ["1 个文件有改动", "2 个文件有改动"]);
        });
    }
}
