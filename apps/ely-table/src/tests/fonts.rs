//! The format panel's font and size (#86).

use ely_office::panel::{Panel as _, send};

use super::data::{open, quiet, run};
use super::*;

#[test]
fn the_panel_sets_a_cells_font_and_size() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    open(&mut cx, &book);
    run(&mut cx, &book, "cell.goto", &["A2"]);
    update(&mut cx, &book, |book, cx| {
        let tab = book.active_tab().expect("a tab");
        let shown = (tab.view.clone(), tab.doc.clone());
        book.panel.update(cx, |panel, cx| panel.show(Some((&shown.0, &shown.1)), cx));
    });
    let panel = cx.update(|app| book.read(app).panel.clone());
    settle(&cx, |app| !panel.read(app).fonts.is_empty());
    let drawn = panel.clone();
    let (_, shown) = cx.add_window_view(move |_, _| Drawn(drawn));
    shown.run_until_parked();
    assert!(shown.debug_bounds("table-format-font").is_some(), "the font box draws");
    assert!(shown.debug_bounds("table-format-size").is_some(), "the size box draws");
    let state = |app: &gpui::App, uno: &str| panel.read(app).state(uno, app).map(|state| state.to_string());
    let was = cx.update(|app| state(app, ".uno:CharFontName"));
    let font = cx.update(|app| {
        let fonts = &panel.read(app).fonts;
        fonts.iter().rev().find(|font| Some(font.to_string()) != was).expect("another font").to_string()
    });
    cx.update(|app| send(&panel, "format.font", &[&font], app));
    quiet(&cx, &book);
    settle(&cx, |app| state(app, ".uno:CharFontName").as_deref() == Some(font.as_str()));
    cx.update(|app| send(&panel, "format.size", &["17"], app));
    quiet(&cx, &book);
    settle(&cx, |app| state(app, ".uno:FontHeight").as_deref() == Some("17"));
    run(&mut cx, &book, "cell.goto", &["A3"]);
    settle(&cx, |app| state(app, ".uno:FontHeight").as_deref() != Some("17"));
}

/// Draws the format panel alone.
struct Drawn(Entity<crate::format::Panel>);

impl gpui::Render for Drawn {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};
        gpui::div().size_full().child(self.0.clone())
    }
}
