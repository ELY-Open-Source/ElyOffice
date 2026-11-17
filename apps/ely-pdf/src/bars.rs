//! The page stack's scrollbars (#84).

use ely_gpui_component::layout::{ScrollModel, ScrollPlace, ScrollSource, Scrollbar};
use gpui::{App, Axis, Entity, Pixels, Point, point};
use std::rc::Rc;

use crate::view::PdfView;

/// The view's scroll; a drag leaves the kept place.
struct Bar(Entity<PdfView>);

impl ScrollModel for Bar {
    fn place(&self, axis: Axis, cx: &App) -> Option<ScrollPlace> {
        let handle = &self.0.read(cx).scroll;
        let (view, max, offset) = (handle.bounds().size, handle.max_offset(), handle.offset());
        let (span, reach, scrolled) = match axis {
            Axis::Vertical => (view.height, max.y, -offset.y),
            Axis::Horizontal => (view.width, max.x, -offset.x),
        };
        (reach > Pixels::ZERO).then(|| ScrollPlace { span, shown: span / (span + reach), passed: (scrolled / reach).clamp(0.0, 1.0) })
    }

    fn set(&self, axis: Axis, ratio: f32, cx: &mut App) {
        self.0.update(cx, |view, cx| {
            view.forget_place();
            let (max, current) = (view.scroll.max_offset(), view.scroll.offset());
            view.scroll.set_offset(match axis {
                Axis::Vertical => point(current.x, -(max.y * ratio)),
                Axis::Horizontal => point(-(max.x * ratio), current.y),
            });
            cx.notify();
        });
    }

    fn offset(&self, cx: &App) -> Point<Pixels> {
        self.0.read(cx).scroll.offset()
    }
}

/// A bar each way; none where everything shows.
pub(crate) fn bars(view: &Entity<PdfView>) -> [Scrollbar; 2] {
    let kept: Rc<dyn ScrollModel> = Rc::new(Bar(view.clone()));
    [
        Scrollbar::new("pdf-bar-x", ScrollSource::Model(kept.clone()), Axis::Horizontal),
        Scrollbar::new("pdf-bar-y", ScrollSource::Model(kept), Axis::Vertical),
    ]
}
