//! The commit box, and history in the left panel.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, ToggleButton, ToggleItem},
    feedback::{EmptyState, ErrorView},
    forms::{Choice, SearchInput, Select},
    git::{CommitInput, CommitList},
    primitives::IconName,
    theme::{ActiveTheme as _, ControlSize},
};
use gpui::{
    AnyElement, Context, Entity, IntoElement, ParentElement as _, SharedString, Styled as _, div,
    prelude::FluentBuilder as _,
};

use crate::{
    changes::Changes,
    commits::Find,
    tree::{Tree, text},
};

/// Who commits, how Git signs, and the message.
pub fn commit_box(
    tree: &Entity<Tree>,
    branch: SharedString,
    staged: usize,
    cx: &mut Context<Changes>,
) -> AnyElement {
    let read = tree.read(cx);
    let colors = cx.theme().colors.clone();
    let (who, signs) = match read.active().and_then(|opened| opened.me.as_ref()) {
        Some(Ok(me)) => {
            let who = match (&me.name, &me.email) {
                (Some(name), Some(email)) => format!("{name} <{email}>").into(),
                _ => text(cx, "tree.me.unset", &[]),
            };
            let signs = me
                .signs
                .as_deref()
                .map(|format| text(cx, "tree.me.signs", &[("format", format)]));
            (who, signs)
        }
        Some(Err(error)) => (text(cx, "tree.me.unread", &[("error", error)]), None),
        None => (SharedString::default(), None),
    };
    let (me, amend, commit) = (tree.downgrade(), tree.downgrade(), tree.downgrade());
    let input = CommitInput::new("tree-commit", &read.subject, &read.body, branch, staged)
        .amend(read.amend)
        .on_amend(move |on, _, cx| {
            amend.update(cx, |tree, cx| tree.set_amend(on, cx)).ok();
        })
        .when(!read.amend_loading, |input| {
            input.on_commit(move |_, cx| {
                commit.update(cx, |tree, cx| tree.commit(cx)).ok();
            })
        });
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_2()
        .border_t_1()
        .border_color(colors.border)
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    Button::new("tree-me", who)
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Sm)
                        .on_click(move |_, window, cx| {
                            me.update(cx, |tree, cx| tree.ask_identity(window, cx)).ok();
                        }),
                )
                .children(signs.map(|signs| div().text_color(colors.fg_muted).child(signs))),
        )
        .child(input)
        .into_any_element()
}

fn find_key(find: Find) -> &'static str {
    match find {
        Find::Message => "message",
        Find::Author => "author",
        Find::Hash => "hash",
        Find::Content => "content",
    }
}

/// The search row, then the commits.
pub fn history(tree: &Entity<Tree>, show_graph: bool, cx: &mut Context<Changes>) -> AnyElement {
    let read = tree.read(cx);
    let Some(opened) = read.active() else {
        return div().into_any_element();
    };
    let (id, hist) = (opened.id, &opened.hist);
    let choices = ["message", "author", "hash", "content"]
        .map(|key| Choice::new(key, text(cx, &format!("tree.find.{key}"), &[])));
    let finding = tree.downgrade();
    let find = Select::new("tree-find", choices)
        .label(text(cx, "tree.find", &[]))
        .size(ControlSize::Sm)
        .selected(find_key(read.find))
        .on_change(move |key, _, cx| {
            let find = match key.as_ref() {
                "author" => Find::Author,
                "hash" => Find::Hash,
                "content" => Find::Content,
                _ => Find::Message,
            };
            finding
                .update(cx, |tree, cx| {
                    tree.find = find;
                    tree.search(cx);
                })
                .ok();
        });
    let toggling = cx.weak_entity();
    let graph = ToggleButton::new(
        "tree-graph",
        ToggleItem::new("history-graph")
            .icon(IconName::PanelLeft)
            .tooltip(text(cx, "tree.log.graph", &[])),
        show_graph,
    )
    .size(ControlSize::Sm)
    .disabled(hist.query.is_some())
    .on_toggle(move |on, _, cx| {
        toggling
            .update(cx, |panel, cx| {
                panel.show_graph = on;
                cx.notify();
            })
            .ok();
    });
    let list = match (&hist.error, hist.rows.is_empty()) {
        (Some(error), _) => ErrorView::new(
            "tree-log-error",
            IconName::TriangleAlert,
            text(cx, "tree.log.unread", &[]),
        )
        .detail(error.clone())
        .into_any_element(),
        (None, true) if hist.read && !hist.loading => {
            let key = if hist.query.is_some() {
                "tree.log.none_found"
            } else {
                "tree.log.none"
            };
            EmptyState::new(
                "tree-log-empty",
                IconName::GitCommitHorizontal,
                text(cx, key, &[]),
            )
            .into_any_element()
        }
        _ => {
            let (pick, more) = (tree.downgrade(), tree.downgrade());
            CommitList::new(("tree-log", id), hist.rows.clone())
                .when(show_graph && hist.query.is_none(), |list| list.graph(hist.graph.clone()))
                .when_some(hist.pick.commit.clone(), |list, picked| {
                    list.selected(picked)
                })
                .on_pick(move |commit, _, cx| {
                    pick.update(cx, |tree, cx| tree.pick_commit(commit, cx))
                        .ok();
                })
                .on_end(move |cx| {
                    more.update(cx, |tree, cx| tree.more_log(cx)).ok();
                })
                .into_any_element()
        }
    };
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(
            div().flex().gap_2().p_2().child(graph).child(find).child(
                div()
                    .flex_1()
                    .child(SearchInput::new("tree-search", &read.search).size(ControlSize::Sm)),
            ),
        )
        .child(div().flex_1().min_h_0().child(list))
        .into_any_element()
}
