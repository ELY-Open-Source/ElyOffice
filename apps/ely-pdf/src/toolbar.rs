//! Reading controls around a centered page field.

use ely_gpui_component::{
    buttons::IconButton,
    forms::NumberInput,
    primitives::IconName,
    shell::{Toolbar, ZoomControl},
    theme::ControlSize,
};
use gpui::{Action, Context, InteractiveElement as _, ParentElement as _, Styled as _, div};

use crate::{keys, viewer::Viewer};

impl Viewer {
    pub(crate) fn page_toolbar(&self, cx: &mut Context<Self>) -> Option<Toolbar> {
        let view = self.active_view()?;
        let (current, count, percent) = {
            let read = view.read(cx);
            (read.current + 1, read.sizes.len(), read.percent())
        };
        let page = NumberInput::new(("pdf-page", view.entity_id()), current as f64)
            .range(1.0, count as f64)
            .size(ControlSize::Sm)
            .label(Self::text(cx, "pdf.page.go_to", &[]))
            .on_commit(move |value, _, cx| {
                view.update(cx, |view, cx| {
                    if value.is_finite() && !view.sizes.is_empty() {
                        let page = (value.round() as usize).clamp(1, view.sizes.len()) - 1;
                        view.show(page, 0.0, cx);
                    }
                });
            });
        let button = |id, icon, key, action: Box<dyn Action>, disabled| {
            IconButton::new(id, icon)
                .size(ControlSize::Sm)
                .tooltip(Self::text(cx, key, &[]))
                .disabled(disabled)
                .on_click(move |_, window, cx| window.dispatch_action(action.boxed_clone(), cx))
        };
        let previous = button(
            "pdf-previous",
            IconName::ChevronLeft,
            "pdf.page.previous",
            Box::new(keys::PreviousPage),
            current == 1,
        );
        let next = button(
            "pdf-next",
            IconName::ChevronRight,
            "pdf.page.next",
            Box::new(keys::NextPage),
            current == count,
        );
        let width = button(
            "pdf-fit-width",
            IconName::AlignHorizontalDistributeCenter,
            "pdf.view.fit_width",
            Box::new(keys::FitWidth),
            false,
        );
        let fit = button(
            "pdf-fit-page",
            IconName::Maximize2,
            "pdf.view.fit_page",
            Box::new(keys::FitPage),
            false,
        );
        let navigation = div()
            .debug_selector(|| "pdf-page-navigation".into())
            .flex()
            .items_center()
            .gap_1()
            .child(
                div()
                    .debug_selector(|| "pdf-previous".into())
                    .child(previous),
            )
            .child(
                div()
                    .w_24()
                    .debug_selector(|| "pdf-page-input".into())
                    .child(page),
            )
            .child(Self::text(
                cx,
                "pdf.page.total",
                &[("count", &count.to_string())],
            ))
            .child(div().debug_selector(|| "pdf-next".into()).child(next));
        let this = cx.entity().downgrade();
        let zoom = ZoomControl::new("pdf-zoom").owned(percent, move |step, _, cx| {
            if let Err(error) = this.update(cx, |viewer, cx| keys::zoom(viewer, step, cx)) {
                log::warn!("pdf: zoom without a viewer: {error}");
            }
        });
        let tools = div()
            .flex_1()
            .min_w_0()
            .flex()
            .justify_end()
            .items_center()
            .gap_1()
            .child(div().debug_selector(|| "pdf-zoom".into()).child(zoom))
            .child(div().debug_selector(|| "pdf-fit-width".into()).child(width))
            .child(div().debug_selector(|| "pdf-fit-page".into()).child(fit));
        Some(
            Toolbar::new()
                .child(div().flex_1().min_w_0())
                .child(navigation)
                .child(tools),
        )
    }
}
