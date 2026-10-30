use std::sync::{Arc, Mutex};

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};

use super::{MarkView, Mode, Reported};
use crate::engine::{Text, mark};
use crate::text::Source;

fn open<'a>(
    text: &str,
    cx: &'a mut TestAppContext,
) -> (Entity<MarkView>, &'a mut VisualTestContext) {
    cx.update(ely_gpui_component::init_for_tests);
    cx.update(|cx| {
        ely_ui::settings::install_defaults(cx);
        ely_ui::i18n::install_as("en-US", cx);
    });
    let source = Source::decode(text.as_bytes().to_vec()).expect("reads");
    let saved = mark(&source.text);
    let doc = Arc::new(Mutex::new(Text::new(source)));
    let (view, cx) = cx.add_window_view(move |window, cx| MarkView::new(doc, saved, window, cx));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    (view, cx)
}

#[gpui::test]
fn typing_reaches_the_text_the_shell_saves(cx: &mut TestAppContext) {
    let (view, cx) = open("# A\n\nB\n", cx);
    let outline: Vec<(usize, String, usize)> = view.read_with(cx, |view, cx| {
        view.outline(cx)
            .into_iter()
            .map(|heading| (heading.level, heading.title, heading.line))
            .collect()
    });
    assert_eq!(outline, [(1, "A".to_string(), 0)]);
    view.update_in(cx, |view, window, cx| view.switch(Mode::Source, window, cx));
    let focus = view.read_with(cx, |view, _| view.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.run_until_parked();
    cx.simulate_input("x");
    let (bytes, modified, changed) = view.read_with(cx, |view, _| {
        (view.bytes(), view.modified(), view.changed())
    });
    assert_eq!(bytes, "x# A\n\nB\n".len());
    assert!(modified && changed > 0);
}

fn current(view: &Entity<MarkView>, cx: &mut VisualTestContext) -> String {
    view.read_with(cx, |view, _| {
        view.doc.lock().expect("the text lock").current.clone()
    })
}

fn focused(view: &Entity<MarkView>, cx: &mut VisualTestContext) {
    let focus = view.read_with(cx, |view, _| view.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.run_until_parked();
}

#[gpui::test]
fn switching_modes_writes_no_byte(cx: &mut TestAppContext) {
    let (view, cx) = open("# A\r\n\r\n* b\r\n", cx);
    for mode in [Mode::Source, Mode::Preview, Mode::Visual, Mode::Source] {
        view.update_in(cx, |view, window, cx| view.switch(mode, window, cx));
        cx.run_until_parked();
    }
    let written = view.read_with(cx, |view, _| {
        let doc = view.doc.lock().expect("the text lock");
        doc.source.encode(&doc.current)
    });
    assert_eq!(written, b"# A\r\n\r\n* b\r\n");
    let (modified, changed) = view.read_with(cx, |view, _| (view.modified(), view.changed()));
    assert!(!modified && changed == 0);
}

#[gpui::test]
fn undo_reaches_across_modes(cx: &mut TestAppContext) {
    let (view, cx) = open("# A\n", cx);
    view.update_in(cx, |view, window, cx| view.switch(Mode::Source, window, cx));
    focused(&view, cx);
    cx.simulate_input("x");
    assert_eq!(current(&view, cx), "x# A\n");
    view.update_in(cx, |view, window, cx| view.switch(Mode::Visual, window, cx));
    focused(&view, cx);
    cx.simulate_keystrokes("secondary-z");
    cx.run_until_parked();
    assert_eq!(
        current(&view, cx),
        "# A\n",
        "the source edit undone in visual"
    );
}

#[gpui::test]
fn undo_brings_back_a_text_emptied_in_source(cx: &mut TestAppContext) {
    let (view, cx) = open("Seed\n", cx);
    view.update_in(cx, |view, window, cx| view.switch(Mode::Source, window, cx));
    view.update(cx, |view, cx| view.adopt(String::new(), cx));
    view.update_in(cx, |view, window, cx| view.switch(Mode::Visual, window, cx));
    focused(&view, cx);
    cx.simulate_keystrokes("secondary-z");
    cx.run_until_parked();
    assert_eq!(current(&view, cx), "Seed\n");
}

#[gpui::test]
fn a_new_empty_file_takes_typing_in_visual(cx: &mut TestAppContext) {
    let (view, cx) = open("", cx);
    assert_eq!(view.read_with(cx, |view, _| view.mode), Mode::Visual);
    focused(&view, cx);
    cx.simulate_input("Hi");
    assert_eq!(current(&view, cx), "Hi");
    assert!(view.read_with(cx, |view, _| view.modified()));
}

#[gpui::test]
fn a_long_file_shows_once_its_blocks_are_read(cx: &mut TestAppContext) {
    let long = "para\n\n".repeat(40_000);
    let (view, cx) = open(&long, cx);
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(1));
    cx.run_until_parked();
    let (shown, read) = view.read_with(cx, |view, cx| {
        (view.shown, view.document.read(cx).read_now(cx))
    });
    assert!(shown && read, "shown with its blocks read");
}

#[gpui::test]
fn a_refresh_is_saved_text_and_keeps_no_undo(cx: &mut TestAppContext) {
    let (view, cx) = open("one\n", cx);
    view.update(cx, |view, cx| view.adopt("edited\n".into(), cx));
    cx.run_until_parked();
    assert!(view.read_with(cx, |view, _| view.modified()));
    let disk = Source::decode(b"from disk\n".to_vec()).expect("reads");
    view.update(cx, |view, cx| view.refresh(disk, cx));
    cx.run_until_parked();
    assert_eq!(current(&view, cx), "from disk\n");
    assert!(!view.read_with(cx, |view, _| view.modified()));
    view.update_in(cx, |view, window, cx| view.switch(Mode::Source, window, cx));
    let focus = view.read_with(cx, |view, _| view.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.run_until_parked();
    cx.simulate_keystrokes("secondary-z");
    cx.run_until_parked();
    assert_eq!(current(&view, cx), "from disk\n", "undo stops at the disk");
}

#[gpui::test]
fn the_shell_hears_a_load_it_subscribed_to(cx: &mut TestAppContext) {
    cx.update(ely_gpui_component::init_for_tests);
    cx.update(|cx| {
        ely_ui::settings::install_defaults(cx);
        ely_ui::i18n::install_as("en-US", cx);
    });
    let heard = std::rc::Rc::new(std::cell::Cell::new(0));
    let (count, held) = (
        heard.clone(),
        std::rc::Rc::new(std::cell::RefCell::new(None)),
    );
    let holder = held.clone();
    let (_root, cx) = cx.add_window_view(move |window, cx| {
        let source = Source::decode(b"# Opened\n".to_vec()).expect("reads");
        let saved = mark(&source.text);
        let doc = Arc::new(Mutex::new(Text::new(source)));
        let view = cx.new(|cx| MarkView::new(doc, saved, window, cx));
        // As the shell: subscribed after the view is made.
        let count = count.clone();
        cx.subscribe(&view, move |_, _, _: &Reported, _| {
            count.set(count.get() + 1)
        })
        .detach();
        holder.replace(Some(view));
        gpui::Empty
    });
    cx.run_until_parked();
    assert!(heard.get() > 0);
}

#[gpui::test]
fn a_text_grown_past_the_limit_goes_to_source(cx: &mut TestAppContext) {
    let (view, cx) = open("# Old\n", cx);
    let long = "x".repeat(ely_gpui_component::documents::LARGEST_READ + 1);
    view.update(cx, |view, cx| view.adopt(long, cx));
    cx.run_until_parked();
    assert!(view.read_with(cx, |view, _| view.large()));
    assert_eq!(view.read_with(cx, |view, _| view.mode), Mode::Source);
    view.update_in(cx, |view, window, cx| {
        view.switch(Mode::Preview, window, cx)
    });
    assert_eq!(view.read_with(cx, |view, _| view.mode), Mode::Source);
    let outline = view.read_with(cx, |view, cx| view.outline(cx));
    assert!(outline.is_empty(), "no stale heading: {outline:?}");
}

#[gpui::test]
fn a_large_file_stays_in_source(cx: &mut TestAppContext) {
    let large = ely_gpui_component::documents::LARGEST_READ / 2 + 1;
    let (view, cx) = open(&"a\n".repeat(large), cx);
    assert_eq!(view.read_with(cx, |view, _| view.mode), Mode::Source);
    view.update_in(cx, |view, window, cx| view.switch(Mode::Visual, window, cx));
    assert_eq!(view.read_with(cx, |view, _| view.mode), Mode::Source);
}

#[gpui::test]
fn an_edit_keeps_the_rest_of_the_file_byte_for_byte(cx: &mut TestAppContext) {
    let bytes = b"\xEF\xBB\xBF# A\r\n\r\nB\r\n".to_vec();
    let (view, cx) = open(std::str::from_utf8(&bytes).expect("UTF-8"), cx);
    view.update_in(cx, |view, window, cx| view.switch(Mode::Source, window, cx));
    let focus = view.read_with(cx, |view, _| view.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.run_until_parked();
    cx.simulate_input("x");
    let written = view.read_with(cx, |view, _| {
        let doc = view.doc.lock().expect("the text lock");
        doc.source.encode(&doc.current)
    });
    assert_eq!(written, b"\xEF\xBB\xBFx# A\r\n\r\nB\r\n");
}

#[gpui::test]
fn a_web_link_opens_outside(cx: &mut TestAppContext) {
    let (view, cx) = open("x\n", cx);
    view.update(cx, |view, cx| view.follow("https://ely.dev", cx));
    assert_eq!(cx.opened_url().as_deref(), Some("https://ely.dev"));
}

#[gpui::test]
fn a_file_link_is_refused_and_said(cx: &mut TestAppContext) {
    let (view, cx) = open("x\n", cx);
    let said = std::rc::Rc::new(std::cell::Cell::new(false));
    let heard = said.clone();
    cx.update(|_, cx| {
        cx.subscribe(&view, move |_, _: &super::Failed, _| heard.set(true))
            .detach()
    });
    view.update(cx, |view, cx| view.follow("file:///etc/hosts", cx));
    cx.run_until_parked();
    assert!(said.get() && cx.opened_url().is_none());
}

/// A GIF whose frame never decodes.
fn broken_gif() -> Vec<u8> {
    let gif =
        "47494638396101000100800000000000ffffff21f90401000000002c0000000001000100000202ffff003b";
    (0..gif.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&gif[at..at + 2], 16).expect("hex"))
        .collect()
}

#[gpui::test]
fn a_broken_gif_ends_and_its_error_names_no_path(cx: &mut TestAppContext) {
    let (_view, cx) = open("x\n", cx);
    let folder = std::env::temp_dir().join(format!("ely-marker-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("a folder");
    let path = folder.join("broken.gif");
    std::fs::write(&path, broken_gif()).expect("the gif");
    let resource = gpui::Resource::Path(path.into());
    let loaded = loop {
        let asked = cx.update(|_, cx| cx.fetch_asset::<gpui::ImageAssetLoader>(&resource));
        if let Some(loaded) = asked {
            break loaded;
        }
        cx.run_until_parked();
    };
    std::fs::remove_dir_all(&folder).expect("cleaned");
    let error = loaded.expect_err("a broken frame fails the load");
    assert!(!format!("{error:?}").contains("ely-marker"), "{error:?}");
}

#[gpui::test]
fn a_broken_inline_gif_ends(cx: &mut TestAppContext) {
    let (_view, cx) = open("x\n", cx);
    let renderer = cx.update(|_, cx| cx.svg_renderer());
    let image = gpui::Image::from_bytes(gpui::ImageFormat::Gif, broken_gif());
    assert!(image.to_image_data(renderer).is_err());
}
