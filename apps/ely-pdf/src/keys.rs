//! Menus and keys of the viewer (PDF-01).

use ely_gpui_component::secondary;
use ely_gpui_component::shell::ZoomStep;
use ely_ui::commands;
use gpui::{
    App, Context, Div, InteractiveElement as _, KeyBinding, Menu, MenuItem, Window, actions,
};

use crate::flip::Way;
use crate::layout::{ACTUAL, Mode, Zoom};
use crate::view::PdfView;
use crate::viewer::Viewer;

actions!(
    pdf,
    [
        Continuous,
        SinglePage,
        TwoPages,
        CoverAlone,
        Horizontal,
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
        ArrowUp,
        ArrowDown,
        ArrowLeft,
        ArrowRight,
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
    ely_office::bind_zooms(cx, (ZoomIn, ZoomOut, ActualSize), at);
    cx.bind_keys([
        KeyBinding::new(secondary!("]"), RotateRight, at),
        KeyBinding::new(secondary!("["), RotateLeft, at),
        KeyBinding::new("pagedown", NextPage, at),
        KeyBinding::new("pageup", PreviousPage, at),
        KeyBinding::new("home", FirstPage, at),
        KeyBinding::new("end", LastPage, at),
        KeyBinding::new("up", ArrowUp, at),
        KeyBinding::new("down", ArrowDown, at),
        KeyBinding::new("left", ArrowLeft, at),
        KeyBinding::new("right", ArrowRight, at),
        KeyBinding::new(secondary!("c"), CopyText, at),
        KeyBinding::new(secondary!("f"), Find, at),
    ]);
}

/// The menus; `book` ticks the cover row.
pub fn menus(book: bool, cx: &App) -> Vec<Menu> {
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
            MenuItem::Action { name: label("pdf.view.cover"), action: Box::new(CoverAlone), os_action: None, checked: book, disabled: false },
            MenuItem::action(label("pdf.view.horizontal"), Horizontal),
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

/// Ticks the cover row when the active tab changed.
pub fn tick_cover(viewer: &mut Viewer, force: bool, cx: &mut Context<Viewer>) {
    let book = viewer.active_view().is_some_and(|view| view.read(cx).mode == Mode::Book);
    if force || viewer.cover_ticked != book {
        viewer.cover_ticked = book;
        ely_ui::set_document_menus(ely_platform::AppId::Pdf, &commands::FILE, menus(book, cx), cx);
    }
}

/// A zoom step: key, menu or toolbar.
pub fn zoom(viewer: &Viewer, step: ZoomStep, cx: &mut App) {
    let to = move |now| match step {
        ZoomStep::In => stepped(now, true),
        ZoomStep::Out => stepped(now, false),
        ZoomStep::Reset => ACTUAL,
    };
    viewer.on_view(|view, cx| view.change(|view, now| view.zoom = Zoom::Scale(to(now)), cx), cx);
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
    let (continuous, single, two, row) = (
        change(|view, _| view.mode = Mode::Continuous),
        change(|view, _| view.mode = Mode::Single),
        change(|view, _| view.mode = Mode::Two),
        change(|view, _| view.mode = Mode::Horizontal),
    );
    let book = change(|view, _| view.mode = if view.mode == Mode::Book { Mode::Two } else { Mode::Book });
    let (width, page) = (change(|view, _| view.zoom = Zoom::Width), change(|view, _| view.zoom = Zoom::Page));
    let (right, left) = (
        change(|view, _| view.rotate = (view.rotate + 90) % 360),
        change(|view, _| view.rotate = (view.rotate + 270) % 360),
    );
    let (next, previous, first, last) = (turn(1), turn(-1), turn(isize::MIN / 2), turn(isize::MAX / 2));
    let arrow = |across, on| move |viewer: &mut Viewer, cx: &mut Context<Viewer>| viewer.on_view(|view, cx| view.arrow(Way { across, on }, cx), cx);
    let (up, down, back, ahead) = (arrow(false, false), arrow(false, true), arrow(true, false), arrow(true, true));
    root.on_action(cx.listener(move |v, _: &Continuous, _, cx| continuous(v, cx)))
        .on_action(cx.listener(move |v, _: &SinglePage, _, cx| single(v, cx)))
        .on_action(cx.listener(move |v, _: &TwoPages, _, cx| two(v, cx)))
        .on_action(cx.listener(move |v, _: &CoverAlone, _, cx| book(v, cx)))
        .on_action(cx.listener(move |v, _: &Horizontal, _, cx| row(v, cx)))
        .on_action(cx.listener(move |v, _: &FitWidth, _, cx| width(v, cx)))
        .on_action(cx.listener(move |v, _: &FitPage, _, cx| page(v, cx)))
        .on_action(cx.listener(|v, _: &ActualSize, _, cx| zoom(v, ZoomStep::Reset, cx)))
        .on_action(cx.listener(|v, _: &ZoomIn, _, cx| zoom(v, ZoomStep::In, cx)))
        .on_action(cx.listener(|v, _: &ZoomOut, _, cx| zoom(v, ZoomStep::Out, cx)))
        .on_action(cx.listener(move |v, _: &RotateRight, _, cx| right(v, cx)))
        .on_action(cx.listener(move |v, _: &RotateLeft, _, cx| left(v, cx)))
        .on_action(cx.listener(move |v, _: &NextPage, _, cx| next(v, cx)))
        .on_action(cx.listener(move |v, _: &PreviousPage, _, cx| previous(v, cx)))
        .on_action(cx.listener(move |v, _: &FirstPage, _, cx| first(v, cx)))
        .on_action(cx.listener(move |v, _: &LastPage, _, cx| last(v, cx)))
        .on_action(cx.listener(move |v, _: &ArrowUp, _, cx| up(v, cx)))
        .on_action(cx.listener(move |v, _: &ArrowDown, _, cx| down(v, cx)))
        .on_action(cx.listener(move |v, _: &ArrowLeft, _, cx| back(v, cx)))
        .on_action(cx.listener(move |v, _: &ArrowRight, _, cx| ahead(v, cx)))
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

    #[test]
    fn zoom_keys_come_from_the_shared_table() {
        let cx = gpui::TestAppContext::single();
        cx.update(|app| {
            super::bind(app);
            let keymap = app.key_bindings();
            let keymap = keymap.borrow();
            let keys = |action: &dyn gpui::Action| -> Vec<String> {
                (keymap.bindings_for_action(action)).map(|binding| binding.keystrokes()[0].inner().key.clone()).collect()
            };
            // The View menu shows = and -, wherever it looks.
            let (zoom_in, zoom_out): (&[&str], &[&str]) = match cfg!(target_os = "macos") {
                true => (&["=", "+", "add"], &["-", "subtract"]),
                false => (&["+", "add", "="], &["subtract", "-"]),
            };
            assert_eq!(keys(&super::ZoomIn), zoom_in);
            assert_eq!(keys(&super::ZoomOut), zoom_out);
        });
    }
}
