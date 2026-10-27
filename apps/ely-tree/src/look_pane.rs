//! The middle for a file's history or blame (TREE-06).

use ely_gpui_component::{
    buttons::{ButtonVariant, IconButton},
    feedback::{EmptyState, ErrorView},
    git::{Blame, BlameView, FileHistory},
    primitives::IconName,
    theme::{ActiveTheme as _, ControlSize},
};
use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement, ParentElement as _, SharedString, Styled as _, div,
    prelude::FluentBuilder as _, relative,
};

use crate::{
    blame::Blamed,
    hist::when,
    look::Look,
    tree::{Side, Tree, text},
};

/// Blame's rows: each owner's words and its age, newest 0.
fn blames(blamed: &Blamed, uncommitted: &SharedString) -> Vec<Blame> {
    let times = blamed
        .owners
        .iter()
        .filter(|owner| !owner.uncommitted())
        .map(|owner| owner.at);
    let (oldest, newest) = (times.clone().min().unwrap_or(0), times.max().unwrap_or(0));
    let span = (newest - oldest).max(1) as f32;
    blamed
        .owners
        .iter()
        .map(|owner| match owner.uncommitted() {
            true => Blame {
                commit: owner.commit.clone().into(),
                author: uncommitted.clone(),
                when: SharedString::default(),
                subject: SharedString::default(),
                age: 0.0,
            },
            false => Blame {
                commit: owner.commit.clone().into(),
                author: owner.author.clone().into(),
                when: when(owner.at),
                subject: owner.subject.clone().into(),
                age: (newest - owner.at) as f32 / span,
            },
        })
        .collect()
}

impl Tree {
    /// Shows a commit from blame in History.
    fn show_commit(&mut self, commit: &str, cx: &mut Context<Self>) {
        self.look = None;
        self.set_side(Side::History, cx);
        self.pick_commit(commit, cx);
    }

    pub fn look_pane(&self, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let colors = cx.theme().colors.clone();
        let (key, path) = match look {
            Look::FileLog { path, .. } => ("tree.file_log.of", path),
            Look::Blame { path, .. } => ("tree.blame.of", path),
        };
        let this = cx.weak_entity();
        let head = div()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1p5()
            .border_b_1()
            .border_color(colors.border)
            .child(div().flex_1().min_w_0().text_color(colors.fg).child(text(
                cx,
                key,
                &[("path", path)],
            )))
            .child(
                IconButton::new("tree-look-close", IconName::X)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .tooltip(text(cx, "tree.close_look", &[]))
                    .on_click(move |_, _, cx| {
                        this.update(cx, |tree, cx| tree.close_look(cx)).ok();
                    }),
            );
        let body = match look {
            Look::FileLog { touches: None, .. } | Look::Blame { read: None, .. } => {
                div().into_any_element()
            }
            Look::FileLog {
                touches: Some(Err(error)),
                ..
            }
            | Look::Blame {
                read: Some(Err(error)),
                ..
            } => ErrorView::new(
                "tree-look-error",
                IconName::TriangleAlert,
                text(cx, "tree.look.unread", &[]),
            )
            .detail(error.clone())
            .into_any_element(),
            Look::FileLog {
                touches: Some(Ok(touches)),
                rows,
                picked,
                view,
                repo,
                ..
            } => {
                let this = cx.weak_entity();
                let list = FileHistory::new(("tree-file-log", *repo), rows.clone())
                    .when_some(
                        picked.map(|at| touches[at].commit.id.clone()),
                        |list, id| list.selected(id),
                    )
                    .on_pick({
                        let ids: Vec<String> = touches
                            .iter()
                            .map(|touch| touch.commit.id.clone())
                            .collect();
                        move |id, _, cx| {
                            if let Some(at) = ids.iter().position(|known| known == id.as_ref()) {
                                this.update(cx, |tree, cx| tree.pick_touch(at, cx)).ok();
                            }
                        }
                    });
                let shown = match picked {
                    Some(at) => {
                        let id = SharedString::from(format!(
                            "tree-file-log-diff-{}",
                            touches[*at].commit.id
                        ));
                        self.view_only(id.into(), &touches[*at].path, view.as_ref(), cx)
                    }
                    None => EmptyState::new(
                        "tree-file-log-none",
                        IconName::History,
                        text(cx, "tree.commit.pick", &[]),
                    )
                    .into_any_element(),
                };
                div()
                    .size_full()
                    .flex()
                    .child(
                        div()
                            .debug_selector(|| "tree-file-log-list".into())
                            .flex_none()
                            .w(relative(0.4))
                            .max_w_96()
                            .h_full()
                            .p_2()
                            .border_r_1()
                            .border_color(colors.border)
                            .child(list),
                    )
                    .child(div().debug_selector(|| "tree-file-log-shown".into()).flex_1().min_w_0().h_full().child(shown))
                    .into_any_element()
            }
            Look::Blame {
                read: Some(Ok(blamed)),
                repo,
                ..
            } => {
                let uncommitted = text(cx, "tree.blame.uncommitted", &[]);
                let this = cx.weak_entity();
                BlameView::new(
                    ("tree-blame", *repo),
                    blamed.code.clone(),
                    blamed.lines.clone(),
                    blames(blamed, &uncommitted),
                )
                .on_commit(move |commit, _, cx| {
                    // Lines not yet committed have no commit to show.
                    if !commit.bytes().all(|b| b == b'0') {
                        this.update(cx, |tree, cx| tree.show_commit(commit, cx))
                            .ok();
                    }
                })
                .into_any_element()
            }
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(head)
            .child(div().flex_1().min_h_0().child(body))
            .into_any_element()
    }
}
