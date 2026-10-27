use ely_gpui_component::theme::{ActiveTheme as _, Mode};
use ely_platform::{
    AppId,
    settings::{Appearance, Shared},
};
use gpui::TestAppContext;

use super::{changed, window::SettingsView};

fn installed(cx: &mut TestAppContext) {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        super::install_defaults(cx);
        crate::theme::install(cx);
        crate::i18n::install(cx);
    });
}

#[gpui::test]
fn a_change_on_disk_turns_the_theme_at_once(cx: &mut TestAppContext) {
    installed(cx);
    let dark = Shared { appearance: Appearance::Dark, ..Shared::default() };
    cx.update(|cx| changed((dark, None), cx));
    cx.run_until_parked();
    cx.update(|cx| {
        assert_eq!(cx.global::<crate::theme::ThemeSettings>().appearance, Appearance::Dark);
        assert_eq!(cx.theme().mode(), Mode::Dark);
    });
    let light = Shared { appearance: Appearance::Light, ..Shared::default() };
    cx.update(|cx| changed((light, None), cx));
    cx.run_until_parked();
    cx.update(|cx| assert_eq!(cx.theme().mode(), Mode::Light));
}

#[gpui::test]
fn the_interface_text_follows_the_settings(cx: &mut TestAppContext) {
    installed(cx);
    let own = cx.update(|cx| cx.theme().font_family.clone());
    let chosen = Shared { ui_font: "Georgia".into(), ui_text_size: 15, ..Shared::default() };
    cx.update(|cx| changed((chosen, None), cx));
    cx.update(|cx| {
        assert_eq!(cx.theme().font_family.as_ref(), "Georgia");
        assert_eq!(cx.theme().font_scale, 15.0 / 13.0);
    });
    cx.update(|cx| changed((Shared::default(), None), cx));
    cx.update(|cx| assert_eq!((cx.theme().font_family.clone(), cx.theme().font_scale), (own, 1.0)));
}

#[gpui::test]
fn a_kept_text_size_applies_at_start(cx: &mut TestAppContext) {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        super::install_defaults(cx);
        cx.global_mut::<super::Current>().shared.ui_text_size = 16;
        crate::theme::install(cx);
        assert_eq!(cx.theme().font_scale, 16.0 / 13.0);
    });
}

#[gpui::test]
fn the_page_scrolls_to_its_last_row_at_the_largest_text(cx: &mut TestAppContext) {
    installed(cx);
    cx.update(crate::updates::offer_for_tests);
    let largest = Shared { ui_text_size: ely_platform::settings::UI_TEXT_SIZES.1, ..Shared::default() };
    cx.update(|cx| changed((largest, None), cx));
    let settings = cx.add_window(|window, cx| SettingsView::new(AppId::Mark, window, cx));
    let mut shown = gpui::VisualTestContext::from_window(settings.into(), cx);
    let height = gpui::px(520.0);
    shown.simulate_resize(gpui::size(gpui::px(720.0), height));
    shown.run_until_parked();
    let before = shown.debug_bounds("settings-check-now").expect("the check is offered");
    assert!(before.bottom() > height, "the page outgrows the window: {before:?}");
    shown.simulate_event(gpui::ScrollWheelEvent {
        position: gpui::point(gpui::px(500.0), gpui::px(300.0)),
        delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.0), gpui::px(-800.0))),
        ..Default::default()
    });
    shown.run_until_parked();
    let after = shown.debug_bounds("settings-check-now").expect("the check is offered");
    assert!(after.bottom() <= height, "scrolled into view: {after:?}");
}

/// The shared file's `key`, as written.
fn kept(key: &str) -> serde_json::Value {
    ely_platform::settings::Store::shared().unwrap().read().unwrap()[key].clone()
}

/// The font picker's pick.
fn font(family: &str, cx: &mut gpui::App) {
    super::picks::pick(ely_platform::settings::UI_FONT, serde_json::json!(family), cx);
}

/// A settings window, drawn; its writes still wait.
fn opened(cx: &mut TestAppContext) -> gpui::VisualTestContext {
    let settings = cx.add_window(|window, cx| SettingsView::new(AppId::Mark, window, cx));
    let mut shown = gpui::VisualTestContext::from_window(settings.into(), cx);
    shown.update(|window, cx| window.draw(cx).clear(cx));
    shown
}

/// Clicks a size segment and draws; no write runs.
fn press(shown: &mut gpui::VisualTestContext, size: u32) {
    let named = ["segment 12", "segment 13", "segment 14", "segment 15", "segment 16"];
    let at = shown.debug_bounds(named[(size - ely_platform::settings::UI_TEXT_SIZES.0) as usize]).expect("a size segment").center();
    shown.update(|window, cx| {
        let down = gpui::MouseDownEvent { position: at, button: gpui::MouseButton::Left, click_count: 1, ..Default::default() };
        let up = gpui::MouseUpEvent { position: at, button: gpui::MouseButton::Left, click_count: 1, ..Default::default() };
        window.dispatch_event(gpui::PlatformInput::MouseDown(down), cx);
        window.dispatch_event(gpui::PlatformInput::MouseUp(up), cx);
    });
    shown.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui::test(iterations = 8)]
fn the_latest_text_pick_is_kept_across_windows(cx: &mut TestAppContext) {
    installed(cx);
    let root = tempfile::tempdir().unwrap().keep();
    // SAFETY: only this test writes the shared file.
    unsafe { std::env::set_var("ELY_DATA_ROOT", &root) };
    let mut shown = opened(cx);
    for size in [14, 15, 16] {
        press(&mut shown, size);
    }
    shown.update(|_, cx| ["Georgia", "Consolas", "Arial"].map(|family| font(family, cx)));
    cx.run_until_parked();
    assert_eq!((kept("ui_text_size"), kept("ui_font")), (serde_json::json!(16), serde_json::json!("Arial")));
    // Back to the size on disk while a write waits.
    press(&mut shown, 13);
    press(&mut shown, 16);
    cx.run_until_parked();
    assert_eq!(kept("ui_text_size"), serde_json::json!(16));
    // Picks still waiting outlive their window.
    press(&mut shown, 14);
    shown.update(|_, cx| font("Georgia", cx));
    shown.update(|window, _| window.remove_window());
    let mut reopened = opened(cx);
    press(&mut reopened, 15);
    reopened.update(|_, cx| font("Consolas", cx));
    cx.run_until_parked();
    assert_eq!((kept("ui_text_size"), kept("ui_font")), (serde_json::json!(15), serde_json::json!("Consolas")));
    // A folder as the lock: writes fail, reads work.
    let lock = ely_platform::settings::Store::shared().unwrap().path().with_file_name(".settings.lock");
    std::fs::remove_file(&lock).unwrap();
    std::fs::create_dir(&lock).unwrap();
    let state = |cx: &mut gpui::VisualTestContext| {
        cx.update(|_, cx| (cx.global::<super::Current>().unsaved.contains_key(ely_platform::settings::UI_TEXT_SIZE), super::picks::waiting(ely_platform::settings::UI_TEXT_SIZE, cx)))
    };
    for size in [14, 13, 16] {
        press(&mut reopened, size);
    }
    let zoom = ely_platform::settings::DOC_ZOOM;
    reopened.update(|_, cx| super::picks::pick(zoom, serde_json::json!(150), cx));
    cx.run_until_parked();
    assert_eq!(kept("ui_text_size"), serde_json::json!(15));
    assert_eq!(state(&mut reopened), (true, Some(serde_json::json!(16))), "the failure shows and the pick stays");
    // Mended, another pick saves and clears both.
    std::fs::remove_dir(&lock).unwrap();
    press(&mut reopened, 14);
    cx.run_until_parked();
    assert_eq!(kept("ui_text_size"), serde_json::json!(14));
    assert_eq!(state(&mut reopened), (false, None));
    // The size's save clears only the size's failure.
    let zoom_failed = reopened.update(|_, cx| (cx.global::<super::Current>().unsaved.contains_key(zoom), super::picks::waiting(zoom, cx)));
    assert_eq!(zoom_failed, (true, Some(serde_json::json!(150))));
    // Unreadable: older picks skip, only the latest fails.
    let file = ely_platform::settings::Store::shared().unwrap().path().to_path_buf();
    std::fs::write(&file, "{").unwrap();
    for size in [15, 16, 13] {
        press(&mut reopened, size);
    }
    cx.run_until_parked();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "{");
    assert_eq!(state(&mut reopened), (true, Some(serde_json::json!(13))), "a skipped write hides no failure");
}

#[gpui::test]
fn the_two_sided_row_picks_what_is_chosen(cx: &mut TestAppContext) {
    installed(cx);
    let two_sided = ely_platform::settings::TWO_SIDED;
    // Printer's, one, long, short; Mark opens on Doc's pick.
    for (app, steps, chosen) in [(AppId::Doc, 2, "long"), (AppId::Mark, 1, "short")] {
        let settings = cx.add_window(|window, cx| SettingsView::new(app, window, cx));
        let mut shown = gpui::VisualTestContext::from_window(settings.into(), cx);
        settings.update(&mut shown, |view, _, _| view.section = "documents".into()).unwrap();
        shown.update(|window, cx| window.draw(cx).clear(cx));
        let at = shown.debug_bounds("settings-two-sided").expect("the row is shown").center();
        // No write runs.
        shown.update(|window, cx| {
            let down = gpui::MouseDownEvent { position: at, button: gpui::MouseButton::Left, click_count: 1, ..Default::default() };
            let up = gpui::MouseUpEvent { position: at, button: gpui::MouseButton::Left, click_count: 1, ..Default::default() };
            window.dispatch_event(gpui::PlatformInput::MouseDown(down), cx);
            window.dispatch_event(gpui::PlatformInput::MouseUp(up), cx);
        });
        for key in std::iter::repeat_n("down", steps).chain(["enter"]) {
            shown.update(|window, cx| window.draw(cx).clear(cx));
            let keystroke = gpui::Keystroke::parse(key).unwrap();
            shown.update(|window, cx| window.dispatch_event(gpui::PlatformInput::KeyDown(gpui::KeyDownEvent { keystroke, is_held: false, prefer_character_input: false }), cx));
        }
        let picked = shown.update(|_, cx| super::picks::waiting(two_sided, cx));
        assert_eq!(picked, Some(serde_json::json!(chosen)), "{app:?}");
        shown.update(|window, _| window.remove_window());
    }
}

#[gpui::test]
fn each_app_shows_its_own_sections(cx: &mut TestAppContext) {
    installed(cx);
    for (app, keys) in [
        (AppId::Doc, &["general", "documents", "storage", "diagnostics"][..]),
        (AppId::Pdf, &["general", "documents", "storage", "diagnostics"]),
        (AppId::Tree, &["general", "git", "storage", "diagnostics"]),
        (AppId::Mark, &["general", "documents", "storage", "diagnostics"]),
        (AppId::Mail, &["general", "mail", "storage", "diagnostics"]),
    ] {
        // As each app does before its first window.
        cx.update(|cx| {
            let menus = crate::commands::menus(app, &crate::commands::OPENING, false, Vec::new(), cx);
            cx.set_menus(menus);
        });
        let window = cx.add_window(|window, cx| SettingsView::new(app, window, cx));
        let shown = window
            .update(cx, |view, _, cx| view.sections(cx).into_iter().map(|(key, ..)| key).collect::<Vec<_>>())
            .unwrap();
        assert_eq!(shown, keys, "{app:?}");
    }
}

/// A main window's frame.
struct Main(gpui::FocusHandle);

impl gpui::Render for Main {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        crate::shell::Shell::new("main", &self.0)
    }
}

/// How far below the title bar the content starts.
fn below_title(cx: &mut TestAppContext, window: gpui::AnyWindowHandle) -> gpui::Pixels {
    let mut shown = gpui::VisualTestContext::from_window(window, cx);
    for _ in 0..3 {
        shown.update(|window, _| window.refresh());
        shown.run_until_parked();
    }
    let title = shown.debug_bounds("shell-title").expect("a title bar");
    shown.debug_bounds("shell-body").expect("a body").origin.y - title.bottom()
}

#[gpui::test]
fn only_the_main_window_carries_menus_and_updates(cx: &mut TestAppContext) {
    installed(cx);
    cx.update(|cx| {
        let menus = crate::commands::menus(AppId::Mark, &crate::commands::OPENING, false, Vec::new(), cx);
        cx.set_menus(menus);
    });
    let main = cx.add_window(|_, cx| Main(cx.focus_handle()));
    let menus = below_title(cx, main.into());
    // macOS keeps menus in the system bar.
    match cfg!(target_os = "macos") {
        true => assert_eq!(menus, gpui::px(0.0), "no menu row on macOS"),
        false => assert!(menus > gpui::px(0.0), "the main window shows the menus"),
    }
    cx.update(crate::updates::offer_for_tests);
    let main = cx.add_window(|_, cx| Main(cx.focus_handle()));
    assert!(below_title(cx, main.into()) > menus, "the main window shows the update");
    let settings = cx.add_window(|window, cx| SettingsView::new(AppId::Mark, window, cx));
    assert_eq!(below_title(cx, settings.into()), gpui::px(0.0), "settings repeats neither");
}

#[gpui::test]
fn the_status_names_this_version_and_the_check(cx: &mut TestAppContext) {
    installed(cx);
    use crate::updates::Checked;
    cx.update(|cx| {
        let line = |checked| super::window::status(AppId::Mark, &checked, cx).to_string();
        let newer = line(Some(Checked::Newer(semver::Version::new(9, 1, 0))));
        assert!(newer.contains("ElyMark") && newer.contains(env!("CARGO_PKG_VERSION")) && newer.contains("9.1.0"), "{newer}");
        let failed = line(Some(Checked::Failed("no route".into())));
        assert!(failed.contains("no route"), "{failed}");
        assert_ne!(line(None), line(Some(Checked::Latest)));
    });
}

/// Where the "Check Now" button sits, if shown.
fn check_button(cx: &mut TestAppContext) -> Option<gpui::Bounds<gpui::Pixels>> {
    let settings = cx.add_window(|window, cx| SettingsView::new(AppId::Mark, window, cx));
    let mut shown = gpui::VisualTestContext::from_window(settings.into(), cx);
    shown.run_until_parked();
    shown.debug_bounds("settings-check-now")
}

#[gpui::test]
fn only_an_installed_app_offers_a_check_and_a_click_starts_one(cx: &mut TestAppContext) {
    installed(cx);
    assert!(check_button(cx).is_none(), "a dev build has no check");
    // SAFETY: tests reading it want this unreachable server.
    unsafe {
        std::env::set_var("ELY_UPDATE_MANIFEST", "http://127.0.0.1:9/none.json");
        std::env::set_var("ELY_UPDATE_PUBLIC_KEY", ely_platform::update::KEY);
    }
    cx.update(crate::updates::offer_for_tests);
    let settings = cx.add_window(|window, cx| SettingsView::new(AppId::Mark, window, cx));
    let mut shown = gpui::VisualTestContext::from_window(settings.into(), cx);
    shown.run_until_parked();
    let button = shown.debug_bounds("settings-check-now").expect("the check is offered");
    shown.simulate_click(button.center(), gpui::Modifiers::default());
    shown.run_until_parked();
    let checked = shown.update(|_, cx| crate::updates::updater(cx).expect("installed").read(cx).checked.clone());
    let Some(crate::updates::Checked::Failed(said)) = checked else {
        panic!("the check ran and failed: {checked:?}");
    };
    assert!(said.starts_with("cannot fetch http://127.0.0.1:9/none.json"), "{said}");
}
