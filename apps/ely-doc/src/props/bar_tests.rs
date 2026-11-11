//! The toolbar runs the kernel's commands (GitHub #67).

use ely_gpui_component::theme::Theme;
use gpui::{Entity, TestAppContext, VisualTestContext};

use crate::docs::Docs;
use crate::tests::{blank, docs, settle, type_into};
use ely_office::prelude::*;

/// The app's window.
struct Host(Entity<Docs>);

impl gpui::Render for Host {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};
        gpui::div().size_full().child(self.0.clone())
    }
}

fn frames(shown: &mut VisualTestContext) {
    // Frames past reduced motion's 1 ms folds.
    for _ in 0..3 {
        std::thread::sleep(std::time::Duration::from_millis(2));
        shown.update(|window, _| window.refresh());
        shown.run_until_parked();
    }
}

/// Clicks `selector` once a frame shows it; states arrive late.
#[track_caller]
fn click(shown: &mut VisualTestContext, selector: &'static str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let bounds = loop {
        frames(shown);
        if let Some(bounds) = shown.debug_bounds(selector) {
            break bounds;
        }
        assert!(std::time::Instant::now() < deadline, "no {selector} in a minute");
    };
    shown.simulate_click(bounds.center(), gpui::Modifiers::none());
    shown.run_until_parked();
}

fn state(app: &gpui::App, docs: &Entity<Docs>, uno: &str) -> Option<String> {
    docs.read(app).active_tab()?.view.read(app).state(uno).map(str::to_string)
}

#[test]
fn the_toolbar_bolds_centers_and_undoes() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        ely_lok::init(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
        ely_ui::set_document_menus(ely_platform::AppId::Doc, &ely_ui::commands::FILE, Vec::new(), cx);
    });
    let id = blank(&mut cx, &docs);
    let window = cx.add_window(|_, _| Host(docs.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    frames(&mut shown);
    assert!(shown.debug_bounds("doc-bar-undo Nothing to undo").is_some(), "Undo says why it is grey");
    assert!(shown.debug_bounds("doc-bar-redo Nothing to redo").is_some(), "Redo says why it is grey");
    type_into(&mut cx, &docs, "Bold");
    cx.update(|app| docs.update(app, |docs, cx| docs.run_active("edit.select_all", &[], cx)));
    shown.update(|window, app| {
        docs.update(app, |docs, cx| {
            docs.own_window(window, cx);
            let focus = docs.tab(id).expect("open").view.read(cx).focus_handle().clone();
            window.focus(&focus, cx);
        })
    });
    settle(&cx, |app| state(app, &docs, ".uno:Bold").as_deref() == Some("false"));
    click(&mut shown, "doc-bar format.bold");
    settle(&cx, |app| state(app, &docs, ".uno:Bold").as_deref() == Some("true"));
    click(&mut shown, "doc-bar paragraph.center");
    settle(&cx, |app| state(app, &docs, ".uno:CenterPara").as_deref() == Some("true"));
    click(&mut shown, "doc-bar-undo Undo");
    settle(&cx, |app| state(app, &docs, ".uno:CenterPara").as_deref() == Some("false"));
    assert_eq!(cx.update(|app| state(app, &docs, ".uno:Bold")).as_deref(), Some("true"), "one step back");
    click(&mut shown, "doc-bar-redo Redo");
    settle(&cx, |app| state(app, &docs, ".uno:CenterPara").as_deref() == Some("true"));
}
