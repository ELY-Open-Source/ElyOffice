//! The Chinese panel shows no English (I-03).

use std::sync::Arc;

use ely_gpui_component::{i18n::I18n, theme::Theme};
use ely_office::laid::LaidOut;
use ely_office::panel::Panel as _;
use gpui::{Entity, TestAppContext, TestDispatcher, VisualTestContext};

use super::Props;
use super::builtin::{self, CHARACTER, PAGE, PARAGRAPH};
use super::tests::panel;
use crate::tests::{blank, docs, settle, type_into};

/// The panel, drawn alone.
struct Shown(Entity<Props>);

impl gpui::Render for Shown {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};
        gpui::div().w(gpui::px(320.0)).h(gpui::px(2400.0)).child(self.0.clone())
    }
}

#[test]
fn the_page_and_style_groups_speak_chinese() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let laid = Arc::new(LaidOut::default());
    let mut cx = TestAppContext::build_with_text_system(TestDispatcher::new(0), None, laid.clone());
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(|app| {
        ely_gpui_component::init_for_tests(app);
        Theme::update(app, |theme| theme.reduced_motion = true);
        I18n::set_locale("zh-CN", app);
    });
    blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "文字");
    let props = panel(&mut cx, &docs);
    cx.update(|app| {
        let shown = props.read(app).shown.as_ref().expect("shown");
        for (names, styles) in [(&shown.paragraph, &PARAGRAPH[..]), (&shown.character, &CHARACTER), (&shown.pages, &PAGE)] {
            for name in names {
                assert!(styles.iter().any(|(kernel, ..)| *kernel == name.as_ref()), "{name} has no Chinese");
            }
        }
    });
    let window = cx.add_window(|_, _| Shown(props.clone()));
    let mut drawn = VisualTestContext::from_window(window.into(), &cx);
    for _ in 0..3 {
        drawn.update(|window, _| window.refresh());
        drawn.run_until_parked();
    }
    let titles = cx.update(|app| ["doc.props.page", "doc.props.style"].map(|key| crate::docs::Docs::text(app, key, &[])));
    for title in titles {
        assert!(laid.saw(&title), "the panel drew {title}");
    }
    assert_eq!(laid.english(&[]), Default::default());
    cx.update(|app| props.update(app, |props, cx| props.set_paper("Letter", cx)));
    settle(&cx, |app| props.read(app).sheet(app).is_some_and(|sheet| sheet.size.0 == 21_590));
    drawn.update(|window, _| window.refresh());
    drawn.run_until_parked();
    // LibreOffice's Chinese keeps the code.
    assert!(laid.saw("美国信函 (Letter)"), "the paper reads in Chinese");
    // A font's name is its own.
    let font = cx.update(|app| props.read(app).state(".uno:CharFontName", app)).expect("a font");
    let mut allowed: Vec<&str> = font.split(' ').collect();
    allowed.push("Letter");
    assert_eq!(laid.english(&allowed), Default::default());
}

#[gpui::test]
fn builtin_styles_read_in_the_interface_language(cx: &mut TestAppContext) {
    cx.update(|app| {
        ely_ui::settings::install_defaults(app);
        ely_ui::i18n::install_as("zh-CN", app);
        assert_eq!(builtin::listed(&PARAGRAPH, "Default Paragraph Style"), "Standard");
        assert_eq!(builtin::listed(&PARAGRAPH, "Ely Note"), "Ely Note");
        assert_eq!(builtin::shown(&PARAGRAPH, "Text body", app), "正文");
        assert_eq!(builtin::shown(&PAGE, "Standard", app), "默认页面样式");
        assert_eq!(builtin::shown(&CHARACTER, "Standard", app), "无字符样式");
        assert_eq!(builtin::shown(&PARAGRAPH, "Ely Note", app), "Ely Note", "a user's style");
        ely_ui::i18n::install_as("en-US", app);
        assert_eq!(builtin::shown(&PARAGRAPH, "Text body", app), "Body Text");
    });
}
