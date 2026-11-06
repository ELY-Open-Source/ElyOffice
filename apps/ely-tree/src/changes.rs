//! The left panel: the work tree's files, or history.

use ely_gpui_component::{
    forms::Choice,
    git::{Changed, ChangesList},
    lists::GitStatus,
    navigation::Tabs,
    primitives::IconName,
};
use gpui::{
    AnyElement, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    StatefulInteractiveElement as _, Styled as _, Subscription, WeakEntity, Window, div,
    prelude::FluentBuilder as _,
};

use crate::{
    status::{Lines, State, Status},
    tree::{Side, Tree, text},
    work::Write,
};

/// The left panel; it reads the active repository.
pub struct Changes {
    tree: WeakEntity<Tree>,
    _follow: Subscription,
    pub show_graph: bool,
}

impl Changes {
    pub fn new(tree: &Entity<Tree>, cx: &mut Context<Self>) -> Self {
        Changes {
            tree: tree.downgrade(),
            _follow: cx.observe(tree, |_, _, cx| cx.notify()),
            show_graph: true,
        }
    }
}

fn badge(letter: char) -> GitStatus {
    match letter {
        'A' | 'C' => GitStatus::Added,
        'D' => GitStatus::Deleted,
        'R' => GitStatus::Renamed,
        _ => GitStatus::Modified,
    }
}

/// The five sections: conflicted, staged, unstaged, untracked, ignored.
pub fn sections(status: &Status) -> [Vec<Changed>; 5] {
    let mut sections: [Vec<Changed>; 5] = Default::default();
    let file = |path: &str, status, lines: Option<&Lines>| Changed {
        path: path.to_string().into(),
        status,
        lines: lines.and_then(|lines| lines.get(path).copied().flatten()),
    };
    for entry in &status.entries {
        let path = entry.path.as_str();
        match &entry.state {
            State::Conflicted(_) => sections[0].push(file(path, GitStatus::Conflicted, None)),
            State::Tracked { staged, unstaged } => {
                if *staged != '.' {
                    sections[1].push(file(path, badge(*staged), Some(&status.staged)));
                }
                if *unstaged != '.' {
                    sections[2].push(file(path, badge(*unstaged), Some(&status.unstaged)));
                }
            }
            State::Untracked => sections[3].push(file(path, GitStatus::Untracked, None)),
            State::Ignored => sections[4].push(file(path, GitStatus::Untracked, None)),
        }
    }
    sections
}

impl Render for Changes {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(tree) = self.tree.upgrade() else {
            return div().into_any_element();
        };
        let side = tree.read(cx).side;
        let panel = match side {
            Side::Changes => changes(&tree, cx),
            Side::History => crate::side_log::history(&tree, self.show_graph, cx),
            Side::Branches => crate::side_refs::branches(&tree, cx),
            Side::Places => crate::side_places::places(&tree, cx),
        };
        let this = tree.downgrade();
        let tabs = [
            Choice::new("changes", text(cx, "tree.side.changes", &[])).icon(IconName::SquarePen),
            Choice::new("history", text(cx, "tree.side.history", &[])).icon(IconName::History),
            Choice::new("branches", text(cx, "tree.side.branches", &[])).icon(IconName::GitBranch),
            Choice::new("places", text(cx, "tree.side.places", &[])).icon(IconName::FolderOpen),
        ];
        let chosen = match side {
            Side::Changes => "changes",
            Side::History => "history",
            Side::Branches => "branches",
            Side::Places => "places",
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                Tabs::new("tree-side", tabs, chosen).on_change(move |value, _, cx| {
                    let side = match value.as_ref() {
                        "history" => Side::History,
                        "branches" => Side::Branches,
                        "places" => Side::Places,
                        _ => Side::Changes,
                    };
                    this.update(cx, |tree, cx| tree.set_side(side, cx)).ok();
                }),
            )
            .child(div().flex_1().min_h_0().child(panel))
            .into_any_element()
    }
}

/// The work tree's files, then the commit box.
fn changes(tree: &Entity<Tree>, cx: &mut Context<Changes>) -> AnyElement {
    let read = tree.read(cx);
    let Some((id, Some(Ok(status)))) = read.active().map(|opened| (opened.id, &opened.status))
    else {
        return div().into_any_element();
    };
    let [conflicted, staged, unstaged, untracked, ignored] = sections(status);
    let staged_count = staged.len();
    let branch = crate::view::branch(&status.head, cx);
    let (on_file, on_all) = (tree.downgrade(), tree.downgrade());
    let list = ChangesList::new(("tree-changes", id), staged, unstaged)
        .conflicted(conflicted)
        .untracked(untracked)
        .ignored(ignored)
        .when_some(
            read.shown.as_ref().filter(|shown| shown.repo == id),
            |list, shown| list.selected(shown.section, shown.path.clone()),
        )
        .on_action(move |path, section, action, _, cx| {
            on_file
                .update(cx, |tree, cx| tree.file_action(path, section, action, cx))
                .ok();
        })
        .on_all(move |stage, _, cx| {
            let write = if stage {
                Write::StageAll
            } else {
                Write::UnstageAll
            };
            on_all.update(cx, |tree, cx| tree.write(id, write, cx)).ok();
        });
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(
            div()
                .id("tree-changes-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .p_2()
                .child(list),
        )
        .child(crate::side_log::commit_box(tree, branch, staged_count, cx))
        .into_any_element()
}
