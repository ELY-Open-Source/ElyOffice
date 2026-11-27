//! Words that say who changed them, under the pointer (#122).

use ely_gpui_component::primitives::Tooltip;
use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, div,
};

use super::LokView;
use crate::Rect;

impl LokView {
    /// Document rects and the words a pointer over each shows.
    pub fn tip_rects(&mut self, tips: Vec<(Rect, SharedString)>, cx: &mut Context<Self>) {
        if self.tips != tips {
            log::info!("lok: {} rects carry a tip", tips.len());
            self.tips = tips;
            cx.notify();
        }
    }

    /// The tips set last.
    #[cfg(any(test, feature = "test-support"))]
    pub fn tipped(&self) -> &[(Rect, SharedString)] {
        &self.tips
    }

    /// A hover area over each rect on screen.
    pub(super) fn tip_areas(&self) -> Vec<AnyElement> {
        self.tips
            .iter()
            .enumerate()
            .filter(|(_, (rect, _))| self.is_visible(*rect))
            .map(|(at, (rect, tip))| {
                let bounds = self.screen_rect(*rect);
                // The view's own children sit at its origin.
                let local = bounds.origin - self.origin;
                div()
                    .id(("lok-view-tip", at))
                    .absolute()
                    .left(local.x)
                    .top(local.y)
                    .w(bounds.size.width)
                    .h(bounds.size.height)
                    .tooltip(Tooltip::text(tip.clone()))
                    .into_any_element()
            })
            .collect()
    }
}
