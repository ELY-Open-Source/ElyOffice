//! The Chinese panel shows no English (I-03).

use std::sync::Arc;

use ely_gpui_component::{i18n::I18n, theme::Theme};
use ely_office::laid::LaidOut;
use ely_office::panel::Panel as _;
use gpui::{Entity, TestAppContext, TestDispatcher, VisualTestContext};

use super::Props;
use super::builtin::{self, CHARACTER, PAGE, PARAGRAPH};
use super::tests::panel;
use crate::docs::Docs;
use crate::tests::{blank, docs, settle, type_into};

/// The panel, drawn alone.
struct Shown(Entity<Props>);

impl gpui::Render for Shown {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};
        gpui::div().w(gpui::px(320.0)).h(gpui::px(2400.0)).child(self.0.clone())
    }
}

/// A blank document's panel in Chinese.
fn chinese_panel(cx: &mut TestAppContext, dir: &std::path::Path) -> Option<Entity<Props>> {
    let docs = docs(cx, dir)?;
    cx.update(|app| {
        ely_gpui_component::init_for_tests(app);
        Theme::update(app, |theme| theme.reduced_motion = true);
        I18n::set_locale("zh-CN", app);
    });
    blank(cx, &docs);
    type_into(cx, &docs, "文字");
    Some(panel(cx, &docs))
}

fn draw(drawn: &mut VisualTestContext) {
    for _ in 0..3 {
        drawn.update(|window, _| window.refresh());
        drawn.run_until_parked();
    }
}

/// A test app whose laid-out text is kept.
fn recording() -> (Arc<LaidOut>, TestAppContext) {
    let laid = Arc::new(LaidOut::default());
    let cx = TestAppContext::build_with_text_system(TestDispatcher::new(0), None, laid.clone());
    (laid, cx)
}

#[test]
fn the_page_and_style_groups_speak_chinese() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (laid, mut cx) = recording();
    let Some(props) = chinese_panel(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
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
    draw(&mut drawn);
    let titles = cx.update(|app| ["doc.props.page", "doc.props.style"].map(|key| Docs::text(app, key, &[])));
    for title in titles {
        assert!(laid.saw(&title), "the panel drew {title}");
    }
    assert_eq!(laid.english(&[]), Default::default());
    cx.update(|app| props.update(app, |props, cx| props.set_paper("Letter", cx)));
    settle(&cx, |app| props.read(app).sheet(app).is_some_and(|sheet| sheet.size.0 == 21_590));
    draw(&mut drawn);
    // LibreOffice's Chinese keeps the code.
    assert!(laid.saw("美国信函 (Letter)"), "the paper reads in Chinese");
    // A font's name is its own.
    let font = cx.update(|app| props.read(app).state(".uno:CharFontName", app)).expect("a font");
    let mut allowed: Vec<&str> = font.split(' ').collect();
    allowed.push("Letter");
    assert_eq!(laid.english(&allowed), Default::default());
}

#[test]
fn a_users_style_named_like_a_builtin_keeps_its_name() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (laid, mut cx) = recording();
    let Some(props) = chinese_panel(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(|app| props.update(app, |props, cx| props.create_style("Text body", "2", cx)));
    settle(&cx, |app| {
        let props = props.read(app);
        !props.reload && props.shown.as_ref().is_some_and(|shown| shown.paragraph.iter().any(|name| name == "Text body (user)"))
    });
    cx.update(|app| props.update(app, |props, cx| props.run("format.style", &["Text body (user)", "ParagraphStyles"], cx)));
    settle(&cx, |app| props.read(app).state(".uno:StyleApply", app).as_deref() == Some("Text body"));
    // A new window lays out afresh.
    laid.clear();
    let window = cx.add_window(|_, _| Shown(props.clone()));
    let mut drawn = VisualTestContext::from_window(window.into(), &cx);
    draw(&mut drawn);
    assert!(laid.saw("Text body"), "the user's style keeps its name");
    assert!(!laid.saw("正文"), "not the built-in body text");
    assert!(!laid.saw("(user)"), "the suffix stays inside");
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
        // Clashes take LibreOffice's suffix, both ways.
        assert_eq!(builtin::listed(&PARAGRAPH, "Text body"), "Text body (user)");
        assert_eq!(builtin::shown(&PARAGRAPH, "Text body (user)", app), "Text body");
        assert_eq!(builtin::listed(&PARAGRAPH, "Body Text (user)"), "Body Text");
        assert_eq!(builtin::shown(&PARAGRAPH, "Body Text", app), "Body Text (user)");
        assert_eq!(builtin::listed(&PAGE, "Standard"), "Standard (user)");
        assert_eq!(builtin::listed(&CHARACTER, "Standard"), "Standard", "no character pool clash");
        assert_eq!(builtin::listed(&PARAGRAPH, "字 (user)"), "字 (user)", "too short to strip");
        ely_ui::i18n::install_as("en-US", app);
        assert_eq!(builtin::shown(&PARAGRAPH, "Text body", app), "Body Text");
    });
}
