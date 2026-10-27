//! Each workbook's strip shows its own chosen sheet.

use ely_lok::{Sheet, Sheets};
use gpui::{
    Bounds, Context, IntoElement, Modifiers, ParentElement as _, Pixels, Render, RenderOnce as _,
    ScrollDelta, ScrollHandle, ScrollEvent, Styled as _, TestAppContext, TouchPhase,
    VisualTestContext, Window, div, point, px,
};

use super::strip;

struct Books {
    book: u64,
    sheets: Sheets,
    scroll: Option<ScrollHandle>,
}

impl Render for Books {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tabs = strip(self.book, &self.sheets)
            .expect("tabs")
            .render(window, cx)
            .into_any_element();
        let id = gpui::ElementId::from(("table-sheets", self.book));
        let handle = window.use_keyed_state((id, "scroll"), cx, |_, _| ScrollHandle::new());
        self.scroll = Some(handle.read(cx).clone());
        div()
            .w(px(640.))
            .child(div().flex().items_center().px_2().child(tabs))
    }
}

fn settle(cx: &mut VisualTestContext) {
    for _ in 0..5 {
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
    }
    cx.run_until_parked();
}

/// The strip's viewport and the chosen tab in it.
fn place(
    view: &gpui::Entity<Books>,
    cx: &mut VisualTestContext,
) -> (Bounds<Pixels>, Bounds<Pixels>) {
    view.read_with(cx, |view, _| {
        let scroll = view.scroll.as_ref().expect("drawn");
        let mut chosen = scroll
            .bounds_for_item(view.sheets.current)
            .expect("chosen tab");
        chosen.origin += scroll.offset();
        (scroll.bounds(), chosen)
    })
}

fn shown(view: &gpui::Entity<Books>, cx: &mut VisualTestContext) -> bool {
    let (viewport, chosen) = place(view, cx);
    chosen.left() >= viewport.left() && chosen.right() <= viewport.right()
}

#[gpui::test]
fn a_switched_workbook_shows_its_chosen_sheet(cx: &mut TestAppContext) {
    cx.update(ely_gpui_component::init_for_tests);
    let sheets = Sheets {
        current: 29,
        list: (0..30)
            .map(|n| Sheet {
                name: format!("Sheet {n}"),
                visible: true,
            })
            .collect(),
    };
    let (view, cx) = cx.add_window_view(|_, _| Books {
        book: 1,
        sheets,
        scroll: None,
    });
    settle(cx);
    assert!(shown(&view, cx), "the first draw shows the chosen sheet");
    for book in [2, 1, 2] {
        cx.simulate_event(ScrollEvent {
            position: point(px(60.), px(16.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(50_000.))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
            ..Default::default()
        });
        settle(cx);
        assert!(!shown(&view, cx), "the wheel moves the chosen sheet away");
        view.update(cx, |view, cx| {
            view.book = book;
            cx.notify();
        });
        settle(cx);
        assert!(shown(&view, cx), "workbook {book} shows its chosen sheet");
    }
}
