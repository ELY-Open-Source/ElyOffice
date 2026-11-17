//! The cell's font and size (#86).

use ely_lok::Doc;
use ely_office::panel::{font_box, size_box};
use gpui::{AnyElement, App, Context, InteractiveElement as _, IntoElement as _, ParentElement as _, SharedString, Window, div};

use super::{Panel, run};
use crate::book::Book;

impl ely_office::panel::Panel for Panel {
    fn state(&self, uno: &str, cx: &App) -> Option<SharedString> {
        let view = self.shown.as_ref()?.view.read(cx);
        view.state(uno).map(|state| SharedString::from(state.to_string()))
    }

    fn run(&mut self, id: &str, values: &[&str], cx: &mut Context<Self>) {
        run(&self.book, id, values, cx);
    }
}

impl Panel {
    /// The installed fonts, read off the window thread.
    pub(super) fn read_fonts(&mut self, doc: &Doc, cx: &mut Context<Self>) {
        ely_lok::read_off(
            &self.worker,
            doc,
            |worker, doc| ely_lok::commands::fonts(&worker.values(doc, ".uno:CharFontName")?),
            |panel: &mut Self, read, cx| match read {
                Ok(fonts) => {
                    panel.fonts = fonts.into_iter().map(Into::into).collect();
                    cx.notify();
                }
                Err(error) => panel.toast(&format!("the font list is unreadable: {error}"), cx),
            },
            cx,
        );
    }

    /// The font and size boxes.
    pub(super) fn font_rows(&self, window: &mut Window, cx: &mut Context<Self>) -> [AnyElement; 2] {
        let label = |key: &str| Book::text(cx, key, &[]);
        let (font, size) = (label("table.format.font"), label("table.format.size"));
        let font = font_box(self, ("table-format-font", font), &self.fonts, window, cx);
        let size = size_box(self, ("table-format-size", size), window, cx);
        [
            div().debug_selector(|| "table-format-font".into()).child(font).into_any_element(),
            div().debug_selector(|| "table-format-size".into()).child(size).into_any_element(),
        ]
    }
}
