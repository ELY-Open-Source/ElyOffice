use super::*;
use crate::client::tests::input::paragraphs;
use ely_platform::clipboard::{self, Clip};
use gpui::{InputHandler, Keystroke, TestAppContext, Window};

/// Steps the scheduler so queued input reaches LOK.
fn flush(cx: &TestAppContext) {
    for _ in 0..80 {
        cx.executor().tick();
    }
}

/// The document's paragraphs, saved through the shared worker.
fn saved(view: &Entity<LokView>, cx: &mut TestAppContext) -> Vec<String> {
    let out = tempfile::tempdir()
        .expect("a temp dir")
        .keep()
        .join("out.fodt");
    cx.update(|app| {
        let view = view.read(app);
        let mut worker = view.worker.lock().expect("the worker lock");
        view.doc
            .save_as(&mut worker, &crate::file_url(&out), "fodt")
            .expect("the save works");
    });
    paragraphs(&std::fs::read_to_string(&out).expect("the save reads"))
}

fn typed(key: &'static str) -> Keystroke {
    let mut stroke = Keystroke::parse(key).expect("a keystroke");
    stroke.key_char = Some(key.to_string());
    stroke
}

#[test]
fn typing_and_composing_reach_the_document() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = TestAppContext::single();
    let (view, _) = cx.update(|app| live_view(app, &install));
    cx.update(|app| {
        view.update(app, |view, _| {
            assert!(!view.key_down(&typed("a")), "text waits for the IME");
            view.commit(None, "ab");
            view.compose(None, "n", None);
            view.compose(None, "ni", None);
            view.compose(None, "你", None);
            assert_eq!(view.marked.as_ref().map(|m| m.text.as_str()), Some("你"));
            view.commit(None, "你");
            assert!(view.marked.is_none());
            view.compose(None, "x", None);
            view.compose(None, "", None);
            assert!(view.marked.is_none(), "an empty composition cancels");
            assert!(view.key_down(&Keystroke::parse("enter").expect("enter")));
        })
    });
    flush(&cx);
    assert_eq!(saved(&view, &mut cx), ["ab你", "Hello 世界"]);
}

pub(super) struct Blank;

impl gpui::Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        gpui::Empty
    }
}

#[test]
fn the_platform_ime_callbacks_compose_in_utf16() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = TestAppContext::single();
    let (view, _) = cx.update(|app| live_view(app, &install));
    let window = cx.add_window(|_, _| Blank);
    cx.update_window(window.into(), |_, window, app| {
        let bounds = Bounds::default();
        let mut ime = gpui::ElementInputHandler::new(bounds, view.clone());
        ime.replace_and_mark_text_in_range(None, "nihao", Some(2..2), window, app);
        let selected = ime
            .selected_text_range(false, window, app)
            .expect("a selection");
        assert_eq!(selected.range, 2..2, "the IME caret sits inside");
        let mut adjusted = None;
        let head = ime.text_for_range(0..2, &mut adjusted, window, app);
        assert_eq!(head.as_deref(), Some("ni"));
        assert_eq!(adjusted, Some(0..2));
        assert_eq!(ime.marked_text_range(window, app), Some(0..5));
        ime.replace_and_mark_text_in_range(Some(0..2), "你", Some(1..1), window, app);
        let marked = view.read(app).marked.as_ref().map(|m| m.text.clone());
        assert_eq!(marked.as_deref(), Some("你hao"));
        let selected = ime.selected_text_range(false, window, app).map(|s| s.range);
        assert_eq!(selected, Some(1..1));
        ime.replace_text_in_range(Some(1..4), "好", window, app);
        assert_eq!(
            ime.marked_text_range(window, app),
            None,
            "the commit ends it"
        );
        ime.replace_and_mark_text_in_range(None, "x", None, window, app);
        ime.unmark_text(window, app);
    })
    .expect("the window lives");
    flush(&cx);
    assert_eq!(saved(&view, &mut cx), ["你好xHello 世界"]);
}

#[test]
fn a_cut_keeps_the_text_when_the_clipboard_fails() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = TestAppContext::single();
    let (view, shared) = cx.update(|app| live_view(app, &install));
    let doc = cx.update(|app| view.read(app).doc.clone());
    {
        let mut worker = shared.lock().expect("the worker lock");
        worker
            .run(&doc, "edit.select_all", &[])
            .expect("select all");
        let failed = crate::view::clip::copy_out(&mut worker, &doc, true, |_| {
            Err(std::io::Error::other("the clipboard refused"))
        });
        assert!(failed.is_err(), "the write failure surfaces");
        assert!(
            !worker.copy(&doc).expect("copy runs").is_empty(),
            "the selection stays"
        );
    }
    assert_eq!(saved(&view, &mut cx), ["Hello 世界"], "the text stays");
}

#[test]
fn a_drag_selects_and_events_carry_the_cursor() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = TestAppContext::single();
    let (view, _) = cx.update(|app| live_view(app, &install));
    cx.update(|app| {
        view.update(app, |view, _| {
            // The caret shows only inside the viewport.
            view.viewport = (800.0, 600.0);
            let none = gpui::Modifiers::default();
            let y = px(view.geom.px_of(1500));
            view.mouse(0, point(px(view.geom.px_of(1500)), y), 1, 1, &none);
            view.mouse(2, point(px(view.geom.px_of(4000)), y), 1, 1, &none);
            view.mouse(1, point(px(view.geom.px_of(4000)), y), 1, 1, &none);
        })
    });
    let mut selected = false;
    for _ in 0..40 {
        flush(&cx);
        selected = cx.update(|app| !view.read(app).selection.is_empty());
        if selected {
            break;
        }
    }
    assert!(selected, "the drag selection reaches the view");
    cx.update(|app| {
        let view = view.read(app);
        let bounds = view.cursor_bounds().expect("the cursor arrived");
        assert!(bounds.size.height > px(0.0), "the caret has height");
    });
}

#[test]
fn a_dead_document_surfaces_an_error() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = TestAppContext::single();
    let (view, shared) = cx.update(|app| live_view(app, &install));
    let errors = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    cx.update(|app| {
        let errors = errors.clone();
        app.subscribe(&view, move |_, error: &LokError, _| {
            errors.borrow_mut().push(error.0.clone())
        })
        .detach();
        let doc = view.read(app).doc.clone();
        shared
            .lock()
            .expect("the worker lock")
            .close_doc(doc)
            .expect("the doc closes");
        view.update(app, |view, _| {
            view.key_down(&Keystroke::parse("enter").expect("enter"));
        });
    });
    flush(&cx);
    let errors = errors.borrow();
    assert!(
        errors
            .iter()
            .any(|error| error.contains("no such document")),
        "the failure reaches subscribers: {errors:?}"
    );
}

#[test]
fn the_clipboard_moves_html_tables_and_images() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    if std::env::var_os("ELY_CLIPBOARD_TEST").is_none() {
        eprintln!("SKIP: set ELY_CLIPBOARD_TEST=1; this replaces the clipboard");
        return;
    }
    let mut cx = TestAppContext::single();
    let (view, _) = cx.update(|app| live_view(app, &install));
    let primary = if cfg!(target_os = "macos") {
        "cmd-a"
    } else {
        "ctrl-a"
    };
    cx.update(|app| {
        view.update(app, |view, _| {
            view.key_down(&Keystroke::parse(primary).expect("select all"));
            view.send(crate::view::input::Input::Copy { cut: false });
        })
    });
    flush(&cx);
    let Some(Clip::Html { html, text }) = clipboard::read().expect("the clipboard reads") else {
        panic!("the copy lands as html");
    };
    assert!(
        html.contains("Hello") && text == "Hello 世界",
        "{html} / {text}"
    );

    clipboard::write(&Clip::Html {
        html: "<table><tr><td>left</td><td>right</td></tr></table>".into(),
        text: "left\tright".into(),
    })
    .expect("the table writes");
    cx.update(|app| view.update(app, |view, _| view.send(crate::view::input::Input::Paste)));
    flush(&cx);
    let text = saved(&view, &mut cx);
    assert!(
        text.contains(&"left".to_string()) && text.contains(&"right".to_string()),
        "{text:?}"
    );
    // Plain text from another app, with no HTML beside it.
    clipboard::write(&Clip::Text("outside words".into())).expect("the text writes");
    cx.update(|app| view.update(app, |view, _| view.send(crate::view::input::Input::Paste)));
    flush(&cx);
    let text = saved(&view, &mut cx);
    assert!(text.iter().any(|line| line.contains("outside words")), "{text:?}");

    let mut png = Vec::new();
    image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 0, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("the png encodes");
    clipboard::write(&Clip::Png(png)).expect("the image writes");
    cx.update(|app| view.update(app, |view, _| view.send(crate::view::input::Input::Paste)));
    flush(&cx);
    let out = tempfile::tempdir()
        .expect("a temp dir")
        .keep()
        .join("img.fodt");
    cx.update(|app| {
        let view = view.read(app);
        let mut worker = view.worker.lock().expect("the worker lock");
        view.doc
            .save_as(&mut worker, &crate::file_url(&out), "fodt")
            .expect("the save works");
    });
    let xml = std::fs::read_to_string(&out).expect("the save reads");
    assert!(xml.contains("<draw:image"), "the image lands");
}

/// Two slides, each with one line of text.
const DECK: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
 <office:body><office:presentation>
  <draw:page draw:name="one"><draw:frame svg:x="2cm" svg:y="2cm" svg:width="10cm" svg:height="3cm"><draw:text-box><text:p>One</text:p></draw:text-box></draw:frame></draw:page>
  <draw:page draw:name="two"><draw:frame svg:x="2cm" svg:y="8cm" svg:width="10cm" svg:height="3cm"><draw:text-box><text:p>Two</text:p></draw:text-box></draw:frame></draw:page>
 </office:presentation></office:body>
</office:document>"#;

/// Paints until tile (0, 0) is fresh, then its bytes.
fn fresh_corner(view: &Entity<LokView>, cx: &mut TestAppContext, not: Option<&[u8]>) -> Vec<u8> {
    let bounds = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(800.0), px(600.0)) };
    for _ in 0..400 {
        let bytes = cx.update(|app| {
            view.update(app, |view, _| view.prepare(bounds));
            let read = view.read(app);
            let entry = read.tiles.get(&(0, 0)).filter(|entry| !entry.stale)?;
            let bytes = entry.image.as_bytes(0).expect("a frame").to_vec();
            (Some(bytes.as_slice()) != not).then_some(bytes)
        });
        if let Some(bytes) = bytes {
            return bytes;
        }
        cx.executor().tick();
    }
    panic!("tile (0, 0) never repainted");
}

#[test]
fn switching_slides_repaints_the_shown_tiles() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = TestAppContext::single();
    let (view, worker) = cx.update(|app| live_view_of(app, &install, "deck.fodp", DECK));
    let one = fresh_corner(&view, &mut cx, None);
    cx.update(|app| view.update(app, |view, _| view.show_part(1)));
    let two = fresh_corner(&view, &mut cx, Some(&one));
    let painted = cx.update(|app| {
        let read = view.read(app);
        let side = read.geom.tile_pixels();
        let mut worker = worker.lock().expect("the worker lock");
        read.doc.paint_tile(&mut worker, (0, 0, TILE_TWIPS, TILE_TWIPS), (side, side)).expect("paints")
    });
    assert_eq!(two, painted.pixels, "the corner shows slide two");
}
