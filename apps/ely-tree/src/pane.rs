//! The middle: the shown file's diff, picture or size (TREE-03).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, IconButton, SegmentedControl},
    data_display::BeforeAfter,
    feedback::{EmptyState, ErrorView},
    git::{ChangeSection, DiffLayout, DiffStat, DiffViewer, LineKind, Stretch},
    primitives::{IconName, Image},
    theme::{ActiveTheme as _, ControlSize},
    typography::{SelectableText, format},
};
use gpui::{
    AnyElement, Context, ElementId, IntoElement, ObjectFit, ParentElement as _, Styled as _, div,
    prelude::FluentBuilder as _,
};

use crate::{
    diff::Space,
    pick::Shown,
    shown::View,
    tree::{Tree, text},
};

fn space_key(space: Space) -> &'static str {
    match space {
        Space::Exact => "exact",
        Space::Changes => "changes",
        Space::All => "all",
    }
}

impl Tree {
    /// The header: path, counts, whitespace, layout, picked lines.
    fn pane_head(&self, shown: &Shown, cx: &mut Context<Self>) -> AnyElement {
        let colors = cx.theme().colors.clone();
        let this = cx.weak_entity();
        let entry_from = self.active().and_then(|opened| match &opened.status {
            Some(Ok(status)) => status
                .entries
                .iter()
                .find(|entry| entry.path == shown.path)?
                .from
                .clone(),
            _ => None,
        });
        let path = match (&entry_from, shown.section) {
            (Some(from), ChangeSection::Staged) => format!("{from} → {}", shown.path),
            _ => shown.path.clone(),
        };
        let stat = match &shown.view {
            Some(Ok(View::Text { stretches, .. })) => Some(count(stretches)),
            _ => None,
        };
        let text_view = matches!(shown.view, Some(Ok(View::Text { .. })));
        let space = SegmentedControl::new("tree-space", space_key(self.space))
            .size(ControlSize::Sm)
            .segment("exact", text(cx, "tree.space.exact", &[]), None)
            .segment("changes", text(cx, "tree.space.changes", &[]), None)
            .segment("all", text(cx, "tree.space.all", &[]), None)
            .on_change({
                let this = this.clone();
                move |key, _, cx| {
                    let space = match key.as_ref() {
                        "changes" => Space::Changes,
                        "all" => Space::All,
                        _ => Space::Exact,
                    };
                    this.update(cx, |tree, cx| tree.set_space(space, cx)).ok();
                }
            });
        let layout = SegmentedControl::new(
            "tree-layout",
            if self.layout == DiffLayout::Split {
                "split"
            } else {
                "unified"
            },
        )
        .size(ControlSize::Sm)
        .segment("unified", text(cx, "tree.layout.unified", &[]), None)
        .segment("split", text(cx, "tree.layout.split", &[]), None)
        .on_change({
            let this = this.clone();
            move |key, _, cx| {
                let layout = if key.as_ref() == "split" {
                    DiffLayout::Split
                } else {
                    DiffLayout::Unified
                };
                this.update(cx, |tree, cx| {
                    tree.layout = layout;
                    cx.notify();
                })
                .ok();
            }
        });
        let key = if shown.section == ChangeSection::Staged {
            "tree.lines.unstage"
        } else {
            "tree.lines.stage"
        };
        let lines = (!shown.picked.is_empty() && self.partial(shown)).then(|| {
            let this = this.clone();
            let n = shown.picked.len();
            Button::new("tree-lines", ely_ui::i18n::counted(cx, key, n as u64, &[("n", &n.to_string())]))
                .variant(ButtonVariant::Primary)
                .size(ControlSize::Sm)
                .on_click(move |_, _, cx| {
                    this.update(cx, |tree, cx| tree.stage_picked(cx)).ok();
                })
        });
        // History and blame need a file Git already tracks.
        let tracked = matches!(
            shown.section,
            ChangeSection::Staged | ChangeSection::Unstaged | ChangeSection::Conflicted
        );
        let looks = tracked.then(|| self.look_buttons(&shown.path, cx));
        let open = {
            let (this, path) = (this.clone(), shown.path.clone());
            IconButton::new("tree-open-file", IconName::ExternalLink)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(text(cx, "tree.open_file", &[]))
                .on_click(move |_, _, cx| {
                    this.update(cx, |tree, cx| tree.open_file(&path, cx)).ok();
                })
        };
        let tools = self.tool_buttons(&shown.path, cx);
        div()
            .flex()
            .items_center()
            .gap_3()
            .px_2()
            .py_1p5()
            .border_b_1()
            .border_color(colors.border)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_color(colors.fg)
                    .child(SelectableText::new("diff-path", path)),
            )
            .children(stat.map(|(added, removed)| DiffStat::new(added, removed)))
            .children(lines)
            .when(text_view, |head| head.child(space).child(layout))
            .children(looks.into_iter().flatten())
            .children(tools)
            .child(open)
            .into_any_element()
    }

    /// Hunks and lines stage only on an exact plain diff.
    fn partial(&self, shown: &Shown) -> bool {
        let staging = matches!(
            shown.section,
            ChangeSection::Staged | ChangeSection::Unstaged
        );
        let plain = matches!(&shown.view, Some(Ok(View::Text { diff, .. })) if diff.partial());
        staging && plain && self.space == Space::Exact
    }

    /// Why hunks and lines do not stage here.
    fn whole_note(&self, shown: &Shown, cx: &mut Context<Self>) -> Option<AnyElement> {
        let staging = matches!(
            shown.section,
            ChangeSection::Staged | ChangeSection::Unstaged | ChangeSection::Untracked
        );
        if !matches!(shown.view, Some(Ok(View::Text { .. }))) || !staging || self.partial(shown) {
            return None;
        }
        let key = if self.space == Space::Exact {
            "tree.whole"
        } else {
            "tree.space.note"
        };
        let colors = cx.theme().colors.clone();
        Some(
            div()
                .px_2()
                .py_1()
                .text_color(colors.fg_muted)
                .child(text(cx, key, &[]))
                .into_any_element(),
        )
    }

    pub fn pane(&self, shown: &Shown, cx: &mut Context<Self>) -> AnyElement {
        if let Some(Ok(View::Conflict(stages))) = &shown.view {
            return self.conflict_pane(&shown.path, stages, cx);
        }
        let body = match &shown.view {
            Some(Ok(View::Text { stretches, .. })) if hunks(stretches) => {
                let this = cx.weak_entity();
                let (open, line, hunk) = (this.clone(), this.clone(), this);
                let mut viewer = DiffViewer::from_stretches(
                    ("tree-diff", shown.repo),
                    shown.path.clone(),
                    stretches.clone(),
                )
                .headless()
                .layout(self.layout)
                .open(shown.opened.clone())
                .on_open(move |stretch, _, cx| {
                    open.update(cx, |tree, cx| {
                        if let Some(shown) = &mut tree.shown {
                            shown.opened.push(stretch);
                            cx.notify();
                        }
                    })
                    .ok();
                });
                if self.partial(shown) {
                    let key = if shown.section == ChangeSection::Staged {
                        "tree.hunk.unstage"
                    } else {
                        "tree.hunk.stage"
                    };
                    viewer = viewer
                        .selected(shown.picked.iter().copied())
                        .on_line(move |place, shift, _, cx| {
                            line.update(cx, |tree, cx| tree.pick_line(place, shift, cx))
                                .ok();
                        })
                        .hunk_action(text(cx, key, &[]), move |stretch, _, cx| {
                            hunk.update(cx, |tree, cx| tree.stage_hunk(stretch, cx))
                                .ok();
                        });
                }
                viewer.into_any_element()
            }
            view => self.still(view.as_ref(), cx),
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(self.pane_head(shown, cx))
            .children(self.whole_note(shown, cx))
            .child(div().flex_1().min_h_0().child(body))
            .into_any_element()
    }

    /// A diff with nothing to stage: commits and file history.
    pub fn view_only(
        &self,
        id: ElementId,
        path: &str,
        view: Option<&Result<View, String>>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match view {
            Some(Ok(View::Text { stretches, .. })) if hunks(stretches) => {
                DiffViewer::from_stretches(id, path.to_string(), stretches.clone())
                    .headless()
                    .layout(self.layout)
                    .into_any_element()
            }
            view => self.still(view, cx),
        }
    }

    /// Everything but a text diff with lines to show.
    fn still(&self, view: Option<&Result<View, String>>, cx: &mut Context<Self>) -> AnyElement {
        match view {
            None => div().into_any_element(),
            Some(Err(error)) => ErrorView::new(
                "tree-diff-error",
                IconName::TriangleAlert,
                text(cx, "tree.diff.unread", &[]),
            )
            .detail(error.clone())
            .into_any_element(),
            Some(Ok(View::Text { .. })) => EmptyState::new(
                "tree-diff-none",
                IconName::Check,
                text(cx, "tree.diff.none", &[]),
            )
            .into_any_element(),
            Some(Ok(View::Picture { old, new })) => {
                let picture = |id: &'static str, image: &std::sync::Arc<gpui::Image>| {
                    Image::new(id, image.clone())
                        .fit(ObjectFit::Contain)
                        .size_full()
                };
                let shown = match (old, new) {
                    (Some(old), Some(new)) => BeforeAfter::new(
                        "tree-picture",
                        picture("tree-old", old),
                        picture("tree-new", new),
                    )
                    .size_full()
                    .into_any_element(),
                    (Some(only), None) | (None, Some(only)) => {
                        picture("tree-only", only).into_any_element()
                    }
                    (None, None) => div().into_any_element(),
                };
                div().size_full().p_4().child(shown).into_any_element()
            }
            Some(Ok(View::Binary { old, new })) => {
                let size = |bytes: &Option<u64>| {
                    bytes.map_or_else(|| "—".to_string(), |bytes| format::file_size(bytes, false))
                };
                let sizes = format!("{} → {}", size(old), size(new));
                EmptyState::new("tree-binary", IconName::File, text(cx, "tree.binary", &[]))
                    .body(sizes)
                    .into_any_element()
            }
            Some(Ok(View::Folder)) => EmptyState::new(
                "tree-folder",
                IconName::Folder,
                text(cx, "tree.folder", &[]),
            )
            .into_any_element(),
            Some(Ok(View::Conflict(_))) => EmptyState::new(
                "tree-conflicted",
                IconName::TriangleAlert,
                text(cx, "tree.conflicted", &[]),
            )
            .into_any_element(),
        }
    }
}

/// Whether Git printed any hunk; a pure rename has none.
fn hunks(stretches: &[Stretch]) -> bool {
    stretches
        .iter()
        .any(|stretch| matches!(stretch, Stretch::Hunk { .. }))
}

/// Lines added and removed in the hunks.
fn count(stretches: &[Stretch]) -> (usize, usize) {
    stretches
        .iter()
        .filter_map(|stretch| match stretch {
            Stretch::Hunk { lines, .. } => Some(lines),
            Stretch::Folded(_) => None,
        })
        .flatten()
        .fold((0, 0), |(added, removed), line| match line.kind {
            LineKind::Added => (added + 1, removed),
            LineKind::Removed => (added, removed + 1),
            LineKind::Same => (added, removed),
        })
}
