//! The Chinese interface shows no English (I-03).

use std::sync::Arc;

use ely_gpui_component::{i18n::I18n, theme::Theme};
use ely_office::laid::LaidOut;
use gpui::{TestDispatcher, VisualTestContext};

use super::data::{asked, open, run};
use super::menu::frames;
use super::*;

/// The open dialog, drawn alone.
struct Shown(Entity<Book>);

impl gpui::Render for Shown {
    fn render(&mut self, _: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};
        let dialog = self.0.update(cx, |book, cx| book.dialog_element(cx));
        gpui::div().size_full().children(dialog)
    }
}

#[test]
fn sort_and_filter_speak_chinese() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let laid = Arc::new(LaidOut::default());
    let mut cx = TestAppContext::build_with_text_system(TestDispatcher::new(0), None, laid.clone());
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(|app| {
        ely_gpui_component::init_for_tests(app);
        Theme::update(app, |theme| theme.reduced_motion = true);
        I18n::set_locale("zh-CN", app);
    });
    open(&mut cx, &book);
    run(&mut cx, &book, "cell.goto", &["A1"]);
    let window = cx.add_window(|_, _| Shown(book.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    for (asked_for, is) in [
        (crate::data::Asked::Sort, (|dialog: &Dialog| matches!(dialog, Dialog::Sort { .. })) as fn(&Dialog) -> bool),
        (crate::data::Asked::Filter, |dialog: &Dialog| matches!(dialog, Dialog::Filter { .. })),
    ] {
        update(&mut cx, &book, |book, cx| book.ask_data(asked_for, cx));
        asked(&cx, &book, is);
        frames(&mut shown);
        // The sheet's own headers are its content.
        assert!(laid.saw("Name"), "{asked_for:?} drew its columns");
        assert_eq!(laid.english(&["Name", "Qty", "City"]), Default::default(), "{asked_for:?}");
    }
}
