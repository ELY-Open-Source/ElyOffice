//! The picked branch or tag, and its actions.

use ely_gpui_component::typography::SelectableText;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    feedback::EmptyState,
    primitives::IconName,
    theme::{ActiveTheme as _, ControlSize},
};
use gpui::{
    AnyElement, App, Context, FontWeight, IntoElement, ParentElement as _, SharedString,
    Styled as _, Window, div,
};

use crate::{
    branches::{Kind, Ref},
    hist::when,
    tree::{Tree, text},
};

type Act = Box<dyn Fn(&mut Tree, &mut Window, &mut Context<Tree>)>;

impl Tree {
    pub fn refs_pane(&self, cx: &mut Context<Self>) -> AnyElement {
        let found = self
            .active()
            .and_then(|opened| match (&opened.refs, &opened.picked_ref) {
                (Some(Ok(refs)), Some((kind, name))) => refs
                    .iter()
                    .find(|r| r.kind == *kind && r.name == *name)
                    .cloned(),
                _ => None,
            });
        let Some(found) = found else {
            return EmptyState::new(
                "tree-ref-none",
                IconName::GitBranch,
                text(cx, "tree.ref.pick", &[]),
            )
            .into_any_element();
        };
        let colors = cx.theme().colors.clone();
        let label = |key: &str| text(cx, key, &[]);
        let mut rows = DescriptionList::new()
            .selectable_text(
                label("tree.ref.commit"),
                format!(
                    "{} {}",
                    &found.commit[..found.commit.len().min(7)],
                    found.subject
                ),
            )
            .selectable_text(
                label("tree.ref.when"),
                found.at.map_or_else(|| "—".into(), when),
            );
        if let Some(upstream) = &found.upstream {
            let track = match found.track {
                Some((ahead, behind)) => format!("{upstream} ↑{ahead} ↓{behind}"),
                None => text(cx, "tree.upstream_gone", &[("upstream", upstream)]).to_string(),
            };
            rows = rows.selectable_text(label("tree.ref.upstream"), track);
        }
        if let Some(message) = &found.message {
            rows = rows.selectable_text(label("tree.ref.message"), message.clone());
        }
        let buttons = self.ref_actions(&found, cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.fg)
                    .child(SelectableText::new("ref-name", found.name.clone())),
            )
            .child(rows)
            .child(div().flex().flex_wrap().gap_2().children(buttons))
            .into_any_element()
    }

    /// What applies to `found`: the current branch takes no switch.
    fn ref_actions(&self, found: &Ref, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let name = found.name.clone();
        let kind = found.kind;
        let at = found.commit.clone();
        let mut acts: Vec<(&str, Act)> = Vec::new();
        let n = name.clone();
        let switchable = match kind {
            Kind::Local => !found.current,
            Kind::Remote => true,
            Kind::Tag => false,
        };
        if switchable {
            acts.push((
                "tree.ref.switch",
                Box::new(move |tree, _, cx| tree.switch(&n, kind, cx)),
            ));
        }
        if !found.current && kind != Kind::Tag {
            // Merge and rebase use full refs, never a same-named tag.
            let (m, r, shown) = (found.full.clone(), found.full.clone(), name.clone());
            acts.push((
                "tree.ref.merge",
                Box::new(move |tree, _, cx| tree.merge(&m, cx)),
            ));
            acts.push((
                "tree.ref.rebase",
                Box::new(move |tree, _, cx| tree.rebase(&r, &shown, cx)),
            ));
        }
        let here = at.clone();
        acts.push((
            "tree.ref.branch_here",
            Box::new(move |tree, window, cx| tree.ask_branch(Some(here.clone()), window, cx)),
        ));
        match kind {
            Kind::Local => {
                let (r, d, f) = (name.clone(), name.clone(), name.clone());
                let (up, gone) = (name.clone(), name.clone());
                acts.push((
                    "tree.upstream.set",
                    Box::new(move |tree, _, cx| tree.ask_upstream(&up, cx)),
                ));
                if found.upstream.is_some() {
                    acts.push((
                        "tree.upstream.unset",
                        Box::new(move |tree, _, cx| tree.unset_upstream(&gone, cx)),
                    ));
                }
                acts.push((
                    "tree.ref.rename",
                    Box::new(move |tree, window, cx| tree.ask_rename(&r, window, cx)),
                ));
                if !found.current {
                    acts.push((
                        "tree.ref.delete",
                        Box::new(move |tree, _, cx| tree.delete_branch(&d, false, cx)),
                    ));
                    acts.push((
                        "tree.ref.force_delete",
                        Box::new(move |tree, _, cx| tree.delete_branch(&f, true, cx)),
                    ));
                }
            }
            Kind::Tag => {
                let d = name.clone();
                acts.push((
                    "tree.ref.delete_tag",
                    Box::new(move |tree, _, cx| tree.delete_tag(&d, cx)),
                ));
            }
            Kind::Remote => {}
        }
        acts.into_iter()
            .map(|(key, act)| {
                let this = cx.weak_entity();
                let destructive = key.contains("delete");
                Button::new(
                    SharedString::from(format!("{key}-button")),
                    text(cx, key, &[]),
                )
                .size(ControlSize::Sm)
                .variant(if destructive {
                    ButtonVariant::Danger
                } else {
                    ButtonVariant::Secondary
                })
                .on_click(move |_, window, cx: &mut App| {
                    this.update(cx, |tree, cx| act(tree, window, cx)).ok();
                })
                .into_any_element()
            })
            .collect()
    }
}
