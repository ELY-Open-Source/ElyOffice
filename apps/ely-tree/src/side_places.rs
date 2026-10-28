//! The Repository panel: stashes, remotes, worktrees, more.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    git::{Stash, StashList},
    lists::{List, ListItem},
    theme::{ActiveTheme as _, ControlSize},
};
use gpui::{
    AnyElement, App, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    Role, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _,
};

use crate::{
    changes::Changes,
    hist::when,
    places::{Place, Read},
    structure::{Checkout, LfsFile},
    tree::{Tree, text},
};

type Act = Box<dyn Fn(&mut Tree, &mut Window, &mut Context<Tree>)>;

fn button(tree: &Entity<Tree>, id: &'static str, words: SharedString, act: Act) -> Button {
    let this = tree.downgrade();
    Button::new(id, words)
        .size(ControlSize::Sm)
        .variant(ButtonVariant::Ghost)
        .on_click(move |_, window, cx| {
            this.update(cx, |tree, cx| act(tree, window, cx)).ok();
        })
}

/// A list's name, with its own buttons.
fn heading(name: SharedString, buttons: Vec<Button>, cx: &App) -> AnyElement {
    let colors = cx.theme().colors.clone();
    div()
        .flex()
        .items_center()
        .gap_1()
        .pt_2()
        .child(
            div()
                .id(SharedString::from(format!("tree-heading-{name}")))
                .role(Role::Heading)
                .aria_label(name.clone())
                .flex_1()
                .text_color(colors.fg_muted)
                .child(name),
        )
        .children(buttons)
        .into_any_element()
}

/// A list Git could not give: its own words.
fn unread(error: &str, cx: &App) -> AnyElement {
    let colors = cx.theme().colors.clone();
    div()
        .text_color(colors.danger)
        .child(error.to_string())
        .into_any_element()
}

fn size(bytes: u64) -> String {
    match bytes {
        0..1024 => format!("{bytes} B"),
        1024..1_048_576 => format!("{:.1} KB", bytes as f64 / 1024.0),
        _ => format!("{:.1} MB", bytes as f64 / 1_048_576.0),
    }
}

fn lfs_row(at: usize, file: &LfsFile, cx: &App) -> AnyElement {
    let key = match (file.checked_out, file.downloaded) {
        (true, _) => "tree.lfs.here",
        (false, true) => "tree.lfs.stored",
        (false, false) => "tree.lfs.pointer",
    };
    ListItem::new(("tree-lfs", at), file.path.clone())
        .description(format!("{} · {}", text(cx, key, &[]), size(file.size)))
        .into_any_element()
}

pub fn places(tree: &Entity<Tree>, cx: &mut Context<Changes>) -> AnyElement {
    let read_tree = tree.read(cx);
    let Some(opened) = read_tree.active() else {
        return div().into_any_element();
    };
    let Some(read) = &opened.places.read else {
        return div().into_any_element();
    };
    let picked = opened.picked_place.clone();
    let label = |key: &str| text(cx, key, &[]);
    let pick = |place: Place| {
        let this = tree.downgrade();
        move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut App| {
            this.update(cx, |tree, cx| tree.pick_place(place.clone(), cx))
                .ok();
        }
    };
    let net = div().flex().flex_wrap().gap_1().children([
        button(
            tree,
            "tree-fetch",
            label("tree.net.fetch"),
            Box::new(|t, _, cx| t.fetch_all(cx)),
        ),
        button(
            tree,
            "tree-pull",
            label("tree.net.pull"),
            Box::new(|t, _, cx| t.pull(cx)),
        ),
        button(
            tree,
            "tree-push",
            label("tree.net.push"),
            Box::new(|t, _, cx| t.push(cx)),
        ),
        button(
            tree,
            "tree-force",
            label("tree.force.open"),
            Box::new(|t, _, cx| t.ask_force(cx)),
        ),
    ]);
    let local = div()
        .flex()
        .flex_wrap()
        .gap_1()
        .children([
            button(
                tree,
                "tree-terminal",
                label("tree.tool.terminal"),
                Box::new(|t, _, cx| t.open_terminal(cx)),
            ),
            button(
                tree,
                "tree-reveal",
                label(crate::tools::REVEAL),
                Box::new(|t, _, cx| t.reveal(cx)),
            ),
            button(
                tree,
                "tree-clean",
                label("tree.clean.open"),
                Box::new(|t, _, cx| t.ask_clean(cx)),
            ),
        ]);
    let mut out: Vec<AnyElement> = vec![net.into_any_element(), local.into_any_element()];
    out.push(heading(
        label("tree.places.stashes"),
        vec![button(
            tree,
            "tree-stash-new",
            label("tree.stash.open"),
            Box::new(|t, window, cx| t.ask_stash(window, cx)),
        )],
        cx,
    ));
    out.push(stashes(tree, read, picked.as_ref(), cx));
    out.push(heading(
        label("tree.places.remotes"),
        vec![button(
            tree,
            "tree-remote-new",
            label("tree.remote.open"),
            Box::new(|t, window, cx| t.ask_remote(None, window, cx)),
        )],
        cx,
    ));
    out.push(match &read.remotes {
        Err(error) => unread(error, cx),
        Ok(remotes) => List::new()
            .children(remotes.iter().enumerate().map(|(at, remote)| {
                let place = Place::Remote(remote.name.clone());
                ListItem::new(("tree-remote", at), remote.name.clone())
                    .description(remote.urls.join(", "))
                    .selected(picked.as_ref() == Some(&place))
                    .on_click(pick(place))
                    .into_any_element()
            }))
            .into_any_element(),
    });
    let prunable =
        matches!(&read.worktrees, Ok(trees) if trees.iter().any(|t| t.prunable.is_some()));
    let mut tree_buttons = vec![button(
        tree,
        "tree-worktree-new",
        label("tree.worktree.open"),
        Box::new(|t, window, cx| t.add_worktree(window, cx)),
    )];
    if prunable {
        tree_buttons.push(button(
            tree,
            "tree-worktree-prune",
            label("tree.worktree.prune"),
            Box::new(|t, _, cx| t.prune_worktrees(cx)),
        ));
    }
    out.push(heading(label("tree.places.worktrees"), tree_buttons, cx));
    out.push(match &read.worktrees {
        Err(error) => unread(error, cx),
        Ok(trees) => List::new()
            .children(trees.iter().enumerate().map(|(at, worktree)| {
                let place = Place::Worktree(worktree.path.clone());
                let about = match (&worktree.branch, &worktree.head, &worktree.prunable) {
                    (_, _, Some(_)) => label("tree.worktree.prunable"),
                    (Some(branch), _, _) => branch.clone().into(),
                    (None, Some(head), _) => text(
                        cx,
                        "tree.branch.detached",
                        &[("oid", &head[..head.len().min(7)])],
                    ),
                    (None, None, _) => label("tree.worktree.bare"),
                };
                ListItem::new(
                    ("tree-worktree", at),
                    worktree.path.file_name().map_or_else(
                        || worktree.path.display().to_string(),
                        |name| name.to_string_lossy().into_owned(),
                    ),
                )
                .description(about)
                .selected(picked.as_ref() == Some(&place))
                .on_click(pick(place))
                .into_any_element()
            }))
            .into_any_element(),
    });
    match &read.submodules {
        Ok(subs) if subs.is_empty() => {}
        Err(error) => {
            out.push(heading(label("tree.places.submodules"), Vec::new(), cx));
            out.push(unread(error, cx));
        }
        Ok(subs) => {
            out.push(heading(label("tree.places.submodules"), Vec::new(), cx));
            out.push(
                List::new()
                    .children(subs.iter().enumerate().map(|(at, sub)| {
                        let place = Place::Submodule(sub.path.clone());
                        let key = match (&sub.checkout, sub.conflicted) {
                            (_, true) => "tree.submodule.conflicted",
                            (Checkout::Missing, _) => "tree.submodule.missing",
                            (Checkout::At(head), _) if *head == sub.recorded => {
                                "tree.submodule.matches"
                            }
                            (Checkout::At(_), _) => "tree.submodule.differs",
                            (Checkout::Unread(_), _) => "tree.submodule.unread",
                        };
                        ListItem::new(("tree-submodule", at), sub.path.clone())
                            .description(label(key))
                            .selected(picked.as_ref() == Some(&place))
                            .on_click(pick(place))
                            .into_any_element()
                    }))
                    .into_any_element(),
            );
        }
    }
    match &read.lfs {
        Ok(files) if files.is_empty() => {}
        Err(error) => {
            out.push(heading(label("tree.places.lfs"), Vec::new(), cx));
            out.push(unread(error, cx));
        }
        Ok(files) => {
            out.push(heading(label("tree.places.lfs"), Vec::new(), cx));
            out.push(
                List::new()
                    .children(
                        files
                            .iter()
                            .enumerate()
                            .map(|(at, file)| lfs_row(at, file, cx)),
                    )
                    .into_any_element(),
            );
        }
    }
    out.push(heading(label("tree.places.reflog"), Vec::new(), cx));
    out.push(match &read.reflog {
        Err(error) => unread(error, cx),
        Ok(moves) => List::new()
            .children(moves.iter().enumerate().map(|(at, moved)| {
                let chosen = matches!(&picked, Some(Place::Reflog(was)) if was.same(moved));
                ListItem::new(("tree-reflog", at), moved.said.clone())
                    .description(format!("{} · {}", moved.name, when(moved.at)))
                    .selected(chosen)
                    .on_click(pick(Place::Reflog(moved.clone())))
                    .into_any_element()
            }))
            .into_any_element(),
    });
    div()
        .id("tree-places-scroll")
        .size_full()
        .overflow_y_scroll()
        .p_2()
        .flex()
        .flex_col()
        .gap_1()
        .children(out)
        .into_any_element()
}

fn stashes(tree: &Entity<Tree>, read: &Read, picked: Option<&Place>, cx: &App) -> AnyElement {
    let list = match &read.stashes {
        Err(error) => return unread(error, cx),
        Ok(list) => list,
    };
    let rows = list.iter().map(|stash| Stash {
        message: stash.message.clone().into(),
        branch: stash.branch.clone().into(),
        when: when(stash.at),
    });
    let oids: Vec<String> = list.iter().map(|stash| stash.oid.clone()).collect();
    let (pick, act) = (tree.downgrade(), tree.downgrade());
    StashList::new("tree-stashes", rows)
        .when_some(
            picked.and_then(|place| match place {
                Place::Stash(oid) => oids.iter().position(|known| known == oid),
                _ => None,
            }),
            |list, at| list.selected(at),
        )
        .on_pick(move |at, _, cx| {
            if let Some(oid) = oids.get(at).cloned() {
                pick.update(cx, |tree, cx| tree.pick_place(Place::Stash(oid), cx))
                    .ok();
            }
        })
        .on_action(move |at, action, _, cx| {
            act.update(cx, |tree, cx| tree.stash_act(at, action, cx))
                .ok();
        })
        .into_any_element()
}
