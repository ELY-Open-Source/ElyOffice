//! Menus and keys of the viewer (PDF-01).

use ely_ui::commands;
use gpui::{
    App, Context, Div, InteractiveElement as _, KeyBinding, Menu, MenuItem, Window, actions,
};

use crate::layout::{ACTUAL, Mode, Zoom};
use crate::view::PdfView;
use crate::viewer::Viewer;

actions!(
    pdf,
    [
        Continuous,
        SinglePage,
        TwoPages,
        ZoomIn,
        ZoomOut,
        FitWidth,
        FitPage,
        ActualSize,
        RotateLeft,
        RotateRight,
        NextPage,
        PreviousPage,
        FirstPage,
        LastPage,
        CopyText,
        Find
    ]
);

/// Zoom steps, in pixels a point.
const STEPS: [f32; 9] = [0.5, 0.67, 0.8, 1.0, ACTUAL, 1.6, 2.0, 3.0, 4.0];

pub fn bind(cx: &mut App) {
    crate::organize::bind(cx);
    crate::writes::bind(cx);
    let at = Some(commands::CONTEXT);
    cx.bind_keys([
        KeyBinding::new("secondary-=", ZoomIn, at),
        KeyBinding::new("secondary--", ZoomOut, at),
        KeyBinding::new("secondary-0", ActualSize, at),
        KeyBinding::new("secondary-]", RotateRight, at),
        KeyBinding::new("secondary-[", RotateLeft, at),
        KeyBinding::new("pagedown", NextPage, at),
        KeyBinding::new("pageup", PreviousPage, at),
        KeyBinding::new("home", FirstPage, at),
        KeyBinding::new("end", LastPage, at),
        KeyBinding::new("secondary-c", CopyText, at),
        KeyBinding::new("secondary-f", Find, at),
    ]);
}

pub fn menus(cx: &App) -> Vec<Menu> {
    let label = |key: &str| Viewer::text(cx, key, &[]);
    vec![
        Menu::new(label("menu.edit")).items(crate::organize::history_items(cx).into_iter().chain([
            MenuItem::separator(),
            MenuItem::action(label("pdf.copy"), CopyText),
            MenuItem::action(label("pdf.find"), Find),
            MenuItem::separator(),
        ]).chain(crate::writes::items(cx))),
        Menu::new(label("menu.view")).items([
            MenuItem::action(label("pdf.view.continuous"), Continuous),
            MenuItem::action(label("pdf.view.single"), SinglePage),
            MenuItem::action(label("pdf.view.two"), TwoPages),
            MenuItem::separator(),
            MenuItem::action(label("pdf.view.zoom_in"), ZoomIn),
            MenuItem::action(label("pdf.view.zoom_out"), ZoomOut),
            MenuItem::action(label("pdf.view.fit_width"), FitWidth),
            MenuItem::action(label("pdf.view.fit_page"), FitPage),
            MenuItem::action(label("pdf.view.actual"), ActualSize),
            MenuItem::separator(),
            MenuItem::action(label("pdf.view.rotate_left"), RotateLeft),
            MenuItem::action(label("pdf.view.rotate_right"), RotateRight),
            MenuItem::separator(),
            ely_ui::commands::left_panel_item(cx),
            ely_ui::commands::right_panel_item(cx),
        ]),
        crate::organize::menu(cx),
    ]
    .into_iter()
    .chain(crate::marking::menus(cx))
    .chain([crate::document::menu(cx)])
    .collect()
}

/// The next zoom step up or down from `now`.
pub fn stepped(now: f32, up: bool) -> f32 {
    match up {
        true => STEPS.into_iter().find(|step| *step > now + 0.001).unwrap_or(STEPS[STEPS.len() - 1]),
        false => STEPS.into_iter().rev().find(|step| *step < now - 0.001).unwrap_or(STEPS[0]),
    }
}

/// Wires every viewer action onto the root.
pub fn on(root: Div, cx: &mut Context<Viewer>) -> Div {
    let change = |change: fn(&mut PdfView, f32)| {
        move |viewer: &mut Viewer, cx: &mut Context<Viewer>| viewer.on_view(|view, cx| view.change(change, cx), cx)
    };
    let turn = |by: isize| move |viewer: &mut Viewer, cx: &mut Context<Viewer>| viewer.on_view(|view, cx| view.turn(by, cx), cx);
    let (continuous, single, two) = (
        change(|view, _| view.mode = Mode::Continuous),
        change(|view, _| view.mode = Mode::Single),
        change(|view, _| view.mode = Mode::Two),
    );
    let (width, page, actual) = (
        change(|view, _| view.zoom = Zoom::Width),
        change(|view, _| view.zoom = Zoom::Page),
        change(|view, _| view.zoom = Zoom::Scale(ACTUAL)),
    );
    let (zoom_in, zoom_out) = (
        change(|view, now| view.zoom = Zoom::Scale(stepped(now, true))),
        change(|view, now| view.zoom = Zoom::Scale(stepped(now, false))),
    );
    let (right, left) = (
        change(|view, _| view.rotate = (view.rotate + 90) % 360),
        change(|view, _| view.rotate = (view.rotate + 270) % 360),
    );
    let (next, previous, first, last) = (turn(1), turn(-1), turn(isize::MIN / 2), turn(isize::MAX / 2));
    root.on_action(cx.listener(move |v, _: &Continuous, _, cx| continuous(v, cx)))
        .on_action(cx.listener(move |v, _: &SinglePage, _, cx| single(v, cx)))
        .on_action(cx.listener(move |v, _: &TwoPages, _, cx| two(v, cx)))
        .on_action(cx.listener(move |v, _: &FitWidth, _, cx| width(v, cx)))
        .on_action(cx.listener(move |v, _: &FitPage, _, cx| page(v, cx)))
        .on_action(cx.listener(move |v, _: &ActualSize, _, cx| actual(v, cx)))
        .on_action(cx.listener(move |v, _: &ZoomIn, _, cx| zoom_in(v, cx)))
        .on_action(cx.listener(move |v, _: &ZoomOut, _, cx| zoom_out(v, cx)))
        .on_action(cx.listener(move |v, _: &RotateRight, _, cx| right(v, cx)))
        .on_action(cx.listener(move |v, _: &RotateLeft, _, cx| left(v, cx)))
        .on_action(cx.listener(move |v, _: &NextPage, _, cx| next(v, cx)))
        .on_action(cx.listener(move |v, _: &PreviousPage, _, cx| previous(v, cx)))
        .on_action(cx.listener(move |v, _: &FirstPage, _, cx| first(v, cx)))
        .on_action(cx.listener(move |v, _: &LastPage, _, cx| last(v, cx)))
        .on_action(cx.listener(|v, _: &CopyText, _, cx| v.on_view(|view, cx| view.copy(cx), cx)))
        .on_action(cx.listener(|v: &mut Viewer, _: &Find, window: &mut Window, cx| v.open_find(window, cx)))
}

#[cfg(test)]
mod tests {
    #[test]
    fn zoom_steps_go_up_and_down_and_stop_at_the_ends() {
        assert_eq!(super::stepped(1.0, true), super::ACTUAL);
        assert_eq!(super::stepped(1.2, false), 1.0, "from between steps");
        assert_eq!(super::stepped(4.0, true), 4.0);
        assert_eq!(super::stepped(0.5, false), 0.5);
    }
}
