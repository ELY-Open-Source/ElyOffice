//! A conflict's middle, and the running operation's alert.

use ely_gpui_component::typography::SelectableText;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    feedback::{Alert, EmptyState},
    git::ThreeWayMerge,
    primitives::{IconName, Severity},
    theme::{ActiveTheme as _, ControlSize},
};
use gpui::{
    AnyElement, App, Context, IntoElement, ParentElement as _, SharedString, Styled as _, Window,
    div,
};

use crate::{
    conflict::Stages,
    ops::{Doing, Op},
    tree::{Tree, text},
};

type Act = Box<dyn Fn(&mut Tree, &mut Context<Tree>)>;

/// Words for ours and theirs, as this operation means.
pub fn sides(doing: Option<&Doing>, cx: &App) -> [SharedString; 2] {
    match doing {
        Some(doing) => {
            let (ours, theirs) = match doing.op {
                Op::Merge => ("tree.ours.merge", "tree.theirs.merge"),
                Op::Rebase => ("tree.ours.rebase", "tree.theirs.rebase"),
                Op::CherryPick => ("tree.ours.pick", "tree.theirs.pick"),
                Op::Revert => ("tree.ours.revert", "tree.theirs.revert"),
                Op::Am => ("tree.ours.am", "tree.theirs.am"),
            };
            [
                text(cx, ours, &[("name", &doing.ours)]),
                text(cx, theirs, &[("name", &doing.theirs)]),
            ]
        }
        None => [text(cx, "tree.ours", &[]), text(cx, "tree.theirs", &[])],
    }
}

fn button(
    id: &str,
    key: &str,
    variant: ButtonVariant,
    act: Act,
    cx: &mut Context<Tree>,
) -> AnyElement {
    let this = cx.weak_entity();
    Button::new(SharedString::from(id.to_string()), text(cx, key, &[]))
        .size(ControlSize::Sm)
        .variant(variant)
        .on_click(move |_, _: &mut Window, cx: &mut App| {
            this.update(cx, |tree, cx| act(tree, cx)).ok();
        })
        .into_any_element()
}

impl Tree {
    /// A stopped operation: continue, skip or abort.
    pub fn op_alert(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let doing = match self.active().map(|opened| opened.doing.as_ref()) {
            Some(Some(Ok(Some(doing)))) => doing,
            // A read that failed says so in Git's own words.
            Some(Some(Err(error))) => {
                let title = text(cx, "tree.op.unread", &[("error", error)]);
                return Some(Alert::new("tree-op", Severity::Danger, title).into_any_element());
            }
            _ => return None,
        };
        let op = doing.op;
        let key = match op {
            Op::Merge => "tree.op.merging",
            Op::Rebase => "tree.op.rebasing",
            Op::CherryPick => "tree.op.picking",
            Op::Revert => "tree.op.reverting",
            Op::Am => "tree.op.applying",
        };
        let title = text(cx, key, &[("ours", &doing.ours), ("theirs", &doing.theirs)]);
        let mut alert = Alert::new("tree-op", Severity::Warning, title)
            .body(text(cx, "tree.op.next", &[]))
            .action(button(
                "tree-op-continue",
                "tree.op.continue",
                ButtonVariant::Primary,
                Box::new(move |tree, cx| tree.op_continue(op, cx)),
                cx,
            ));
        if op.skips() {
            alert = alert.action(button(
                "tree-op-skip",
                "tree.op.skip",
                ButtonVariant::Secondary,
                Box::new(move |tree, cx| tree.op_skip(op, cx)),
                cx,
            ));
        }
        Some(
            alert
                .action(button(
                    "tree-op-abort",
                    "tree.op.abort",
                    ButtonVariant::Danger,
                    Box::new(move |tree, cx| tree.op_abort(op, cx)),
                    cx,
                ))
                .into_any_element(),
        )
    }

    pub fn conflict_pane(&self, path: &str, stages: &Stages, cx: &mut Context<Self>) -> AnyElement {
        let colors = cx.theme().colors.clone();
        let doing = self.active().and_then(|opened| match &opened.doing {
            Some(Ok(doing)) => doing.clone(),
            _ => None,
        });
        let [ours, theirs] = sides(doing.as_ref(), cx);
        let (text_view, present) = match stages {
            Stages::Text { .. } => (self.merging.as_ref(), [true; 3]),
            Stages::Whole { present } => (None, *present),
        };
        let mut actions: Vec<(&str, &str, ButtonVariant, Act)> = Vec::new();
        if text_view.is_some() {
            actions.push((
                "tree-merge-save",
                "tree.conflict.save",
                ButtonVariant::Secondary,
                Box::new(|tree, cx| tree.save_merge(cx)),
            ));
        }
        actions.push((
            "tree-merge-done",
            "tree.conflict.resolved",
            ButtonVariant::Primary,
            Box::new(|tree, cx| tree.mark_resolved(cx)),
        ));
        // A side's whole file only where that side has one.
        if present[1] {
            actions.push((
                "tree-merge-ours",
                "tree.conflict.use_ours",
                ButtonVariant::Ghost,
                Box::new(|tree, cx| tree.take_side(true, cx)),
            ));
        }
        if present[2] {
            actions.push((
                "tree-merge-theirs",
                "tree.conflict.use_theirs",
                ButtonVariant::Ghost,
                Box::new(|tree, cx| tree.take_side(false, cx)),
            ));
        }
        if !present[1] || !present[2] {
            actions.push((
                "tree-merge-delete",
                "tree.conflict.delete",
                ButtonVariant::Danger,
                Box::new(|tree, cx| tree.resolve_deleted(cx)),
            ));
        }
        let buttons: Vec<AnyElement> = actions
            .into_iter()
            .map(|(id, key, variant, act)| button(id, key, variant, act, cx))
            .collect();
        let head = div()
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
                    .child(SelectableText::new("conflict-path", path.to_string())),
            )
            .children(buttons);
        let Some(merging) = text_view else {
            let key = match present {
                [_, false, _] => "tree.conflict.gone_ours",
                [_, _, false] => "tree.conflict.gone_theirs",
                _ => "tree.conflict.whole",
            };
            let body = EmptyState::new(
                "tree-conflict-whole",
                IconName::TriangleAlert,
                text(cx, key, &[]),
            )
            .body(format!("{ours} · {theirs}"));
            return div()
                .size_full()
                .flex()
                .flex_col()
                .child(head)
                .child(body)
                .into_any_element();
        };
        let base = text(cx, "tree.base", &[]);
        let this = cx.weak_entity();
        let merge = ThreeWayMerge::new(
            SharedString::from(format!("tree-merge-{path}")),
            [ours.as_ref(), base.as_ref(), theirs.as_ref()],
            merging.regions.clone(),
        )
        .takes(merging.takes.clone())
        .on_take(move |at, take, _, cx| {
            this.update(cx, |tree, cx| tree.take(at, take, cx)).ok();
        });
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(head)
            .child(div().flex_1().min_h_0().p_2().child(merge))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .border_t_1()
                    .border_color(colors.border)
                    .child(self.merge_editor.clone()),
            )
            .into_any_element()
    }
}
