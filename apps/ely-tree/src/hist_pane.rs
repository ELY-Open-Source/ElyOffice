//! The middle in History: the picked commit (TREE-06).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, IconButton},
    data_display::DescriptionList,
    feedback::{EmptyState, ErrorView},
    lists::{List, ListItem},
    primitives::IconName,
    theme::{ActiveTheme as _, ControlSize},
};
use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _,
    Role, SharedString, StatefulInteractiveElement as _, Styled as _, div,
    prelude::FluentBuilder as _, px, relative,
};

use crate::{
    detail::{Detail, Person},
    hist::{Hist, Picked, Whose, when},
    tree::{Tree, text},
};

/// `%G?` in words.
fn signature(letter: char) -> &'static str {
    match letter {
        'G' => "tree.sig.good",
        'B' => "tree.sig.bad",
        'U' => "tree.sig.untrusted",
        'X' => "tree.sig.expired",
        'Y' => "tree.sig.expired_key",
        'R' => "tree.sig.revoked",
        'E' => "tree.sig.unchecked",
        _ => "tree.sig.none",
    }
}

fn person(who: &Person) -> SharedString {
    format!("{} <{}>, {}", who.name, who.email, when(who.at)).into()
}

impl Tree {
    /// A file's history and blame, as two buttons.
    pub fn look_buttons(&self, path: &str, cx: &mut Context<Self>) -> [IconButton; 2] {
        let (log, blame, at, blamed) = (
            cx.weak_entity(),
            cx.weak_entity(),
            path.to_string(),
            path.to_string(),
        );
        [
            IconButton::new("tree-file-log", IconName::History)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(text(cx, "tree.file_log", &[]))
                .on_click(move |_, _, cx| {
                    log.update(cx, |tree, cx| tree.file_log(&at, cx)).ok();
                }),
            IconButton::new("tree-blame", IconName::User)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(text(cx, "tree.blame", &[]))
                .on_click(move |_, _, cx| {
                    blame.update(cx, |tree, cx| tree.blame(&blamed, cx)).ok();
                }),
        ]
    }

    /// A committed file's path, its history and its blame.
    fn file_head(&self, path: &str, cx: &mut Context<Self>) -> AnyElement {
        let colors = cx.theme().colors.clone();
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1p5()
            .border_b_1()
            .border_color(colors.border)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_color(colors.fg)
                    .child(path.to_string()),
            )
            .children(self.look_buttons(path, cx))
            .into_any_element()
    }

    pub fn commit_pane(&self, hist: &Hist, cx: &mut Context<Self>) -> AnyElement {
        self.picked_pane(&hist.pick, Whose::History, cx)
    }

    /// A history commit or a stash, as read so far.
    pub fn picked_pane(&self, picked: &Picked, whose: Whose, cx: &mut Context<Self>) -> AnyElement {
        match (&picked.commit, &picked.detail) {
            (None, _) => EmptyState::new(
                "tree-commit-none",
                IconName::GitCommitHorizontal,
                text(cx, "tree.commit.pick", &[]),
            )
            .into_any_element(),
            (Some(_), None) => div().into_any_element(),
            (Some(_), Some(Err(error))) => ErrorView::new(
                "tree-commit-error",
                IconName::TriangleAlert,
                text(cx, "tree.commit.unread", &[]),
            )
            .detail(error.clone())
            .into_any_element(),
            (Some(_), Some(Ok(detail))) => {
                let head = match whose {
                    Whose::History => self.commit_head(detail, cx),
                    Whose::Stash => self.stash_head(detail, cx),
                    Whose::Reflog => self.reflog_head(detail, cx),
                };
                self.commit_detail(detail, picked, whose, head, cx)
            }
        }
    }

    fn commit_head(&self, detail: &Detail, cx: &mut Context<Self>) -> AnyElement {
        let colors = cx.theme().colors.clone();
        let (subject, body) = detail
            .message
            .split_once('\n')
            .unwrap_or((&detail.message, ""));
        let label = |key: &str| text(cx, key, &[]);
        let mut rows =
            DescriptionList::new().text(label("tree.commit.author"), person(&detail.author));
        if (&detail.committer.name, &detail.committer.email)
            != (&detail.author.name, &detail.author.email)
        {
            rows = rows.text(label("tree.commit.committer"), person(&detail.committer));
        }
        let parents = detail.parents.iter().map(|parent| {
            let (this, id) = (cx.weak_entity(), parent.clone());
            Button::new(
                SharedString::from(format!("tree-parent-{parent}")),
                parent[..parent.len().min(7)].to_string(),
            )
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .on_click(move |_, _, cx| {
                this.update(cx, |tree, cx| tree.pick_commit(&id, cx)).ok();
            })
        });
        let signed = match detail.signer.as_str() {
            "" => label(signature(detail.signed)),
            signer => format!("{}, {signer}", label(signature(detail.signed))).into(),
        };
        rows = rows
            .text(label("tree.commit.id"), detail.id.clone())
            .item(label("tree.commit.parents"), div().flex().children(parents))
            .text(label("tree.commit.signature"), signed);
        if !detail.refs.is_empty() {
            rows = rows.text(label("tree.commit.refs"), detail.refs.clone());
        }
        div()
            .id("tree-commit-head")
            .flex_none()
            .max_h(px(280.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .border_b_1()
            .border_color(colors.border)
            .child(
                div()
                    .id("tree-commit-subject")
                    .role(Role::Heading)
                    .aria_label(SharedString::from(subject.to_string()))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.fg)
                    .child(subject.to_string()),
            )
            .when(!body.trim().is_empty(), |head| {
                let body = SharedString::from(body.trim().to_string());
                head.child(
                    div()
                        .id("tree-commit-body")
                        .role(Role::Label)
                        .aria_label(body.clone())
                        .text_color(colors.fg_muted)
                        .child(body),
                )
            })
            .child(rows)
            .child(self.commit_actions(detail, cx))
            .into_any_element()
    }

    /// Pick, revert, or branch or tag at this commit.
    fn commit_actions(&self, detail: &Detail, cx: &mut Context<Self>) -> AnyElement {
        let (id, parents) = (detail.id.clone(), detail.parents.len());
        type Act = Box<dyn Fn(&mut Tree, &mut gpui::Window, &mut Context<Tree>)>;
        let acts: [(&str, Act); 4] = [
            ("tree.commit.pick_it", {
                let id = id.clone();
                Box::new(move |tree, _, cx| tree.cherry_pick(&id, parents, cx))
            }),
            ("tree.commit.revert_it", {
                let id = id.clone();
                Box::new(move |tree, _, cx| tree.revert(&id, parents, cx))
            }),
            ("tree.ref.branch_here", {
                let id = id.clone();
                Box::new(move |tree, window, cx| tree.ask_branch(Some(id.clone()), window, cx))
            }),
            ("tree.ref.tag_here", {
                let id = id.clone();
                Box::new(move |tree, window, cx| tree.ask_tag(Some(id.clone()), window, cx))
            }),
        ];
        div()
            .flex()
            .flex_wrap()
            .gap_2()
            .children(acts.into_iter().map(|(key, act)| {
                let this = cx.weak_entity();
                Button::new(
                    SharedString::from(format!("{key}-button")),
                    text(cx, key, &[]),
                )
                .size(ControlSize::Sm)
                .on_click(move |_, window, cx| {
                    this.update(cx, |tree, cx| act(tree, window, cx)).ok();
                })
            }))
            .into_any_element()
    }

    fn commit_detail(
        &self,
        detail: &Detail,
        pick: &Picked,
        whose: Whose,
        head: AnyElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = cx.theme().colors.clone();
        let picked = pick.file.as_ref().map(|(at, _)| *at);
        let weak = cx.weak_entity();
        let files: Vec<AnyElement> = detail
            .files
            .iter()
            .enumerate()
            .map(|(at, file)| {
                let this = weak.clone();
                let title = match &file.from {
                    Some(from) => format!("{from} → {}", file.path),
                    None => file.path.clone(),
                };
                let about = match file.lines {
                    Some((added, removed)) => format!("{} · +{added} −{removed}", file.status),
                    None => file.status.to_string(),
                };
                ListItem::new(SharedString::from(format!("tree-touched-{at}")), title)
                    .description(about)
                    .selected(picked == Some(at))
                    .on_click(move |_, _, cx| {
                        this.update(cx, |tree, cx| tree.pick_touched(whose, at, cx))
                            .ok();
                    })
                    .into_any_element()
            })
            .collect();
        let shown = match &pick.file {
            Some((at, view)) => {
                let path = &detail.files[*at].path;
                let diff = self.view_only(
                    SharedString::from(format!("tree-commit-diff-{}", detail.id)).into(),
                    path,
                    view.as_ref(),
                    cx,
                );
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(self.file_head(path, cx))
                    .child(div().flex_1().min_h_0().child(diff))
                    .into_any_element()
            }
            None => EmptyState::new(
                "tree-file-none",
                IconName::FileText,
                text(cx, "tree.commit.pick_file", &[]),
            )
            .into_any_element(),
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(head)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(
                        div()
                            .id("tree-touched")
                            .debug_selector(|| "tree-touched".into())
                            .flex_none()
                            .w(relative(0.4))
                            .max_w_72()
                            .h_full()
                            .overflow_y_scroll()
                            .p_2()
                            .border_r_1()
                            .border_color(colors.border)
                            .child(List::new().children(files)),
                    )
                    .child(div().debug_selector(|| "tree-touched-shown".into()).flex_1().min_w_0().h_full().child(shown)),
            )
            .into_any_element()
    }
}
