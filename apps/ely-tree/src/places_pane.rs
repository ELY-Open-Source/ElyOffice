//! The middle in Repository: the picked stash, remote and more.

use ely_gpui_component::typography::SelectableText;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    feedback::EmptyState,
    git::StashAction,
    primitives::IconName,
    theme::{ActiveTheme as _, ControlSize},
};
use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _,
    Role, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
};

use crate::{
    detail::Detail,
    hist::{Whose, when},
    places::{Place, Read},
    structure::Checkout,
    tree::{Tree, text},
};

type Act = Box<dyn Fn(&mut Tree, &mut Window, &mut Context<Tree>)>;

impl Tree {
    fn acts(&self, acts: Vec<(&'static str, bool, Act)>, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .flex_wrap()
            .gap_2()
            .children(acts.into_iter().map(|(key, danger, act)| {
                let this = cx.weak_entity();
                Button::new(
                    SharedString::from(format!("{key}-button")),
                    text(cx, key, &[]),
                )
                .size(ControlSize::Sm)
                .variant(if danger {
                    ButtonVariant::Danger
                } else {
                    ButtonVariant::Secondary
                })
                .on_click(move |_, window, cx| {
                    this.update(cx, |tree, cx| act(tree, window, cx)).ok();
                })
            }))
            .into_any_element()
    }

    /// A heading, its facts, and what can be done.
    fn card(
        &self,
        title: String,
        rows: DescriptionList,
        acts: AnyElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = cx.theme().colors.clone();
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .child(
                div()
                    .id("tree-place-title")
                    .role(Role::Heading)
                    .aria_label(SharedString::from(title.clone()))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.fg)
                    .child(SelectableText::new("place-title-text", title)),
            )
            .child(rows)
            .child(acts)
            .into_any_element()
    }

    /// A stash's head: its words, when, and its three actions.
    pub fn stash_head(&self, detail: &Detail, cx: &mut Context<Self>) -> AnyElement {
        let at = self
            .stashes()
            .iter()
            .position(|stash| stash.oid == detail.id);
        let subject = detail.message.lines().next().unwrap_or("").to_string();
        let label = |key: &str| text(cx, key, &[]);
        let rows = DescriptionList::new()
            .selectable_text(label("tree.stash.when"), when(detail.author.at))
            .selectable_text(label("tree.commit.id"), detail.id.clone());
        let acts = match at {
            Some(at) => self.acts(
                vec![
                    (
                        "tree.stash.apply",
                        false,
                        Box::new(move |t, _, cx| t.stash_act(at, StashAction::Apply, cx)),
                    ),
                    (
                        "tree.stash.pop",
                        false,
                        Box::new(move |t, _, cx| t.stash_act(at, StashAction::Pop, cx)),
                    ),
                    (
                        "tree.stash.drop",
                        true,
                        Box::new(move |t, _, cx| t.stash_act(at, StashAction::Drop, cx)),
                    ),
                ],
                cx,
            ),
            None => div().into_any_element(),
        };
        let colors = cx.theme().colors.clone();
        div()
            .flex_none()
            .border_b_1()
            .border_color(colors.border)
            .child(self.card(subject, rows, acts, cx))
            .into_any_element()
    }

    /// Where HEAD was, and the two ways back.
    pub fn reflog_head(&self, detail: &Detail, cx: &mut Context<Self>) -> AnyElement {
        // The picked entry itself, while it still names this commit.
        let moved = match (
            self.active().and_then(|o| o.places.read.as_ref()),
            self.active().and_then(|o| o.picked_place.as_ref()),
        ) {
            (
                Some(Read {
                    reflog: Ok(moves), ..
                }),
                Some(Place::Reflog(was)),
            ) => moves
                .iter()
                .find(|m| m.same(was) && m.oid == detail.id)
                .cloned(),
            _ => None,
        };
        let label = |key: &str| text(cx, key, &[]);
        let mut rows = DescriptionList::new();
        if let Some(moved) = &moved {
            rows = rows
                .selectable_text(label("tree.reflog.where"), moved.name.clone())
                .selectable_text(label("tree.reflog.when"), when(moved.at));
        }
        rows = rows.selectable_text(label("tree.commit.id"), detail.id.clone());
        let (branch, back) = (detail.id.clone(), detail.id.clone());
        let acts = self.acts(
            vec![
                (
                    "tree.ref.branch_here",
                    false,
                    Box::new(move |t, window, cx| t.ask_branch(Some(branch.clone()), window, cx)),
                ),
                (
                    "tree.move.open",
                    true,
                    Box::new(move |t, _, cx| t.ask_move(&back, cx)),
                ),
            ],
            cx,
        );
        let title = moved.map_or_else(
            || detail.message.lines().next().unwrap_or("").to_string(),
            |moved| moved.said,
        );
        let colors = cx.theme().colors.clone();
        div()
            .flex_none()
            .border_b_1()
            .border_color(colors.border)
            .child(self.card(title, rows, acts, cx))
            .into_any_element()
    }

    pub fn places_pane(&self, cx: &mut Context<Self>) -> AnyElement {
        let none = || {
            EmptyState::new(
                "tree-place-none",
                IconName::Archive,
                text(cx, "tree.places.pick", &[]),
            )
            .into_any_element()
        };
        let Some(opened) = self.active() else {
            return none();
        };
        let (Some(read), Some(place)) = (&opened.places.read, &opened.picked_place) else {
            return none();
        };
        let label = |key: &str| text(cx, key, &[]);
        match place {
            Place::Stash(oid) if opened.places.pick.commit.as_ref() == Some(oid) => {
                self.picked_pane(&opened.places.pick, Whose::Stash, cx)
            }
            Place::Remote(name) => {
                let Some(remote) = found(read, |read| &read.remotes, |r| &r.name == name) else {
                    return none();
                };
                let push = match remote.push.is_empty() {
                    true => label("tree.remote.same"),
                    false => remote.push.join("\n").into(),
                };
                let rows = DescriptionList::new()
                    .selectable_text(label("tree.remote.url"), remote.urls.join("\n"))
                    .selectable_text(label("tree.remote.push_url"), push);
                let (fetch, edit, remove) = (name.clone(), remote.clone(), name.clone());
                let acts = self.acts(
                    vec![
                        (
                            "tree.net.fetch",
                            false,
                            Box::new(move |t, _, cx| t.fetch_one(&fetch, cx)),
                        ),
                        (
                            "tree.remote.change",
                            false,
                            Box::new(move |t, window, cx| {
                                t.ask_remote(Some(edit.clone()), window, cx)
                            }),
                        ),
                        (
                            "tree.remote.delete",
                            true,
                            Box::new(move |t, _, cx| t.remove_remote(&remove, cx)),
                        ),
                    ],
                    cx,
                );
                self.card(name.clone(), rows, acts, cx)
            }
            Place::Worktree(path) => {
                let Some(tree) = found(read, |read| &read.worktrees, |t| &t.path == path) else {
                    return none();
                };
                let mut rows = DescriptionList::new();
                rows = match (&tree.branch, &tree.head) {
                    (Some(branch), _) => {
                        rows.selectable_text(label("tree.worktree.on"), branch.clone())
                    }
                    (None, Some(head)) => {
                        rows.selectable_text(label("tree.commit.id"), head.clone())
                    }
                    (None, None) => {
                        rows.selectable_text(label("tree.worktree.on"), label("tree.worktree.bare"))
                    }
                };
                if let Some(why) = &tree.locked {
                    rows = rows.selectable_text(label("tree.worktree.locked"), why.clone());
                }
                if let Some(why) = &tree.prunable {
                    rows = rows.selectable_text(label("tree.worktree.prunable"), why.clone());
                }
                // The main worktree and this one stay.
                let main = found(read, |read| &read.worktrees, |_| true)
                    .is_some_and(|first| first.path == tree.path);
                let here = tree.path == opened.repo.root;
                let mut acts: Vec<(&'static str, bool, Act)> = Vec::new();
                if !here && tree.prunable.is_none() && !tree.bare {
                    let open = tree.path.clone();
                    acts.push((
                        "tree.worktree.open_it",
                        false,
                        Box::new(move |t, _, cx| t.open_path(open.clone(), cx)),
                    ));
                }
                if !main && !here {
                    let gone = tree.path.clone();
                    acts.push((
                        "tree.worktree.delete",
                        true,
                        Box::new(move |t, _, cx| t.remove_worktree(&gone, cx)),
                    ));
                }
                let acts = self.acts(acts, cx);
                self.card(tree.path.display().to_string(), rows, acts, cx)
            }
            Place::Submodule(path) => {
                let Some(sub) = found(read, |read| &read.submodules, |s| &s.path == path) else {
                    return none();
                };
                let held = match &sub.checkout {
                    Checkout::Missing => label("tree.submodule.missing"),
                    Checkout::At(head) => head.clone().into(),
                    Checkout::Unread(error) => error.clone().into(),
                };
                let rows = DescriptionList::new()
                    .selectable_text(label("tree.submodule.recorded"), sub.recorded.clone())
                    .selectable_text(label("tree.submodule.held"), held);
                let mut acts: Vec<(&'static str, bool, Act)> = Vec::new();
                if !matches!(&sub.checkout, Checkout::At(head) if *head == sub.recorded) {
                    let update = sub.path.clone();
                    acts.push((
                        "tree.submodule.update",
                        false,
                        Box::new(move |t, _, cx| t.update_submodule(&update, cx)),
                    ));
                }
                if matches!(sub.checkout, Checkout::At(_)) {
                    let open = sub.path.clone();
                    acts.push((
                        "tree.submodule.open",
                        false,
                        Box::new(move |t, _, cx| t.open_submodule(&open, cx)),
                    ));
                }
                let acts = self.acts(acts, cx);
                self.card(sub.path.clone(), rows, acts, cx)
            }
            // Gone from the reflog, the event shows nothing.
            Place::Reflog(was)
                if opened.places.pick.commit.as_ref() == Some(&was.oid)
                    && matches!(&read.reflog, Ok(moves) if moves.iter().any(|m| m.same(was))) =>
            {
                self.picked_pane(&opened.places.pick, Whose::Reflog, cx)
            }
            Place::Stash(_) | Place::Reflog(_) => none(),
        }
    }
}

/// The first item of a list that read, matching `is`.
fn found<'a, T: Clone + 'a>(
    read: &'a Read,
    list: impl Fn(&'a Read) -> &'a Result<Vec<T>, String>,
    is: impl Fn(&T) -> bool,
) -> Option<T> {
    list(read)
        .as_ref()
        .ok()?
        .iter()
        .find(|item| is(item))
        .cloned()
}
