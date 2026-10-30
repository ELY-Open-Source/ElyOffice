use super::*;
use gpui::TestAppContext;

/// Steps the scheduler until `done` or ten seconds pass.
fn settle(cx: &TestAppContext, done: impl Fn(&App) -> bool) -> bool {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if !cx.executor().tick() {
            cx.executor()
                .advance_clock(std::time::Duration::from_millis(20));
        }
        if cx.update(|app| done(app)) {
            return true;
        }
    }
    false
}

/// The view's 800 by 600 pixel area.
fn area() -> Bounds<Pixels> {
    Bounds {
        origin: point(px(0.0), px(0.0)),
        size: size(px(800.0), px(600.0)),
    }
}

/// A user profile whose dark scheme paints the page black.
const DARK_PROFILE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<oor:items xmlns:oor="http://openoffice.org/2001/registry" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<item oor:path="/org.openoffice.Office.UI/ColorScheme/ColorSchemes"><node oor:name="TEST_DARK" oor:op="replace"><node oor:name="DocColor"><prop oor:name="Color" oor:op="fuse"><value>0</value></prop></node></node></item>
</oor:items>
"#;

/// Shows a view in a window.
struct Host(Entity<LokView>);

impl gpui::Render for Host {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        self.0.clone()
    }
}

#[test]
fn a_dark_app_paints_the_page_white() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = TestAppContext::single();
    cx.update(|app| {
        ely_gpui_component::init_for_tests(app);
        ely_gpui_component::theme::Theme::set_mode_now(ely_gpui_component::theme::Mode::Dark, app);
    });
    // A dark profile, as an old Ely left.
    let dir = tempfile::tempdir().expect("a temp dir");
    let profile = dir.path().join("profile");
    std::fs::create_dir_all(profile.join("user")).expect("a profile");
    std::fs::write(profile.join("user/registrymodifications.xcu"), DARK_PROFILE).expect("the scheme writes");
    let source = dir.path().join("note.fodt");
    std::fs::write(&source, crate::client::tests::FODT).expect("the fixture writes");
    let mut started = crate::Worker::spawn(&install, &profile).expect("the worker");
    let doc = started.load(&crate::file_url(&source)).expect("the document loads");
    let laid = super::super::layout(&mut started, &doc).expect("the layout reads");
    let worker = Arc::new(Mutex::new(started));
    let shade = || {
        let mut worker = worker.lock().expect("the worker");
        let tile = doc.paint_tile(&mut worker, (1440, 1440, 3840, 3840), (64, 64)).expect("a tile");
        let sum: u64 = tile.pixels.as_chunks::<4>().0.iter().map(|px| u64::from(px[0]) + u64::from(px[1]) + u64::from(px[2])).sum();
        sum / (64 * 64 * 3)
    };
    worker.lock().expect("the worker").run(&doc, "view.theme", &["TEST_DARK"]).expect("the dark scheme");
    let dark = shade();
    assert!(dark < 100, "the kernel starts dark: {dark}");
    let view = cx.update(|app| LokView::open(Arc::clone(&worker), doc.clone(), laid, app));
    let shown = view.clone();
    cx.add_window(move |_, _| Host(shown));
    assert!(
        settle(&cx, |app| view.read(app).lit() && view.read(app).settled()),
        "the scheme goes out"
    );
    assert_eq!(paper(), gpui::white(), "the page mask is the scheme's white");
    let light = shade();
    assert!(light > 240, "the view paints the page white: {light}");
}

#[test]
fn selection_keeps_document_text_readable_across_themes() {
    use ely_gpui_component::theme::{ActiveTheme, Mode, Palette, Theme, contrast, over};

    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let (view, worker) = cx.update(|app| live_view(app, &install));
    let doc = cx.update(|app| view.read(app).doc.clone());
    let shown = view.clone();
    let window = cx.add_window(move |_, _| Host(shown));
    worker
        .lock()
        .expect("the worker")
        .run(&doc, "edit.select_all", &[])
        .expect("select all");
    assert!(
        settle(&cx, |app| !view.read(app).selection.is_empty()),
        "the selection arrives"
    );

    let mut themes = vec![
        ("dark".to_string(), Mode::Dark, None),
        ("light".to_string(), Mode::Light, None),
    ];
    for (name, mode, color) in [
        ("omarchy dark", Mode::Dark, 0x292e42),
        ("omarchy light", Mode::Light, 0xccd0da),
    ] {
        let mut palette = match mode {
            Mode::Dark => Palette::dark(false),
            Mode::Light => Palette::light(false),
        };
        palette.selection = gpui::rgb(color).into();
        themes.push((name.to_string(), mode, Some(palette)));
    }
    let mut translucent = Palette::dark(false);
    translucent.selection = gpui::rgb(0x292e42).into();
    translucent.selection.a = 0.15;
    themes.push(("translucent".to_string(), Mode::Dark, Some(translucent)));

    for (name, mode, palette) in themes {
        cx.update(|app| {
            Theme::set_palette(mode, palette, app);
            Theme::set_mode_now(mode, app);
        });
        cx.update_window(window.into(), |_, window, app| {
            window.draw(app).clear(app);
            let source = app.theme().colors.selection;
            let selected: Vec<_> = window
                .painted_quads()
                .into_iter()
                .filter_map(|quad| quad.background.as_solid())
                .filter(|color| color.alpha(1.0) == source.alpha(1.0))
                .collect();
            assert!(!selected.is_empty(), "{name}: the selection is painted");
            for color in selected {
                for (text, background) in [(gpui::black(), paper()), (gpui::white(), gpui::black())]
                {
                    let ratio = contrast(over(color, text), over(color, background));
                    assert!(
                        ratio >= 4.5,
                        "{name}: selected text stays readable: {ratio}"
                    );
                }
                assert!(
                    color.a > 0.0 && color.a <= source.a,
                    "{name}: the theme's opacity is respected"
                );
                if source.a < 0.3 {
                    assert_eq!(
                        color.a, source.a,
                        "{name}: a translucent theme stays unchanged"
                    );
                }
            }
        })
        .expect("the window lives");
    }
}

#[test]
fn a_failed_tile_reports_and_retries_on_a_move() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = TestAppContext::single();
    let (view, _) = cx.update(|app| live_view(app, &install));
    let errors = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    cx.update(|app| {
        let errors = errors.clone();
        app.subscribe(&view, move |_, error: &LokError, _| {
            errors.borrow_mut().push(error.0.clone())
        })
        .detach();
        view.update(app, |view, cx| {
            let geom = view.geom;
            view.prepare(area());
            view.fetching.remove(&(0, 0));
            view.finish_tile((0, 0), geom, Err(io::Error::other("no paint")), cx);
            assert!(view.failed.contains(&((0, 0), view.generation)));
            view.prepare(area());
            assert!(!view.fetching.contains(&(0, 0)), "no retry storm in place");
            view.scroll_by(0.0, -40.0, cx);
            view.prepare(area());
            assert!(view.fetching.contains(&(0, 0)), "a move retries the tile");
        });
    });
    assert!(
        settle(&cx, |_| !errors.borrow().is_empty()),
        "the failure reaches the app"
    );
    assert!(
        errors.borrow()[0].contains("no paint"),
        "{:?}",
        errors.borrow()
    );
}
