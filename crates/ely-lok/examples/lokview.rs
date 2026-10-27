//! A live LokView in ElyDoc's shell, driven by input events.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use ely_lok::{Doc, LokView, Worker};
use ely_platform::{AppId, clipboard};
use gpui::{
    App, Bounds, ElementInputHandler, Entity, InputHandler, KeyUpEvent, Keystroke, Modifiers,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, PlatformInput, Point,
    Window,
};

/// Line centers and text edges in twips, measured.
const LINE_ONE: i32 = 1556;
const LINE_TWO: i32 = 1832;
const TEXT_LEFT: i32 = 1420;
const PAST_END: i32 = 8000;

fn main() {
    ely_ui::run(AppId::Doc, |app, window, cx| {
        ely_lok::init(cx);
        let install =
            PathBuf::from(std::env::var("ELY_LOK_DIR").expect("ELY_LOK_DIR names the LOK build"));
        let dir = tempfile::tempdir().expect("a temp dir").keep();
        let mut worker = Worker::spawn(&install, &dir.join("profile")).expect("the worker spawns");
        let source = dir.join("probe.fodt");
        std::fs::write(&source, FODT).expect("the fodt writes");
        let doc = worker
            .load(&format!("file://{}", source.display()))
            .expect("the document loads");
        let worker = Arc::new(Mutex::new(worker));
        let laid = ely_lok::layout(&mut worker.lock().expect("the worker lock"), &doc)
            .expect("the layout reads");
        let view = LokView::open(Arc::clone(&worker), doc.clone(), laid, cx);
        let probe = Probe {
            view: view.clone(),
            worker,
            doc,
            dir,
        };
        window.on_next_frame(move |window, cx| probe.step(0, 0, window, cx));
        ely_ui::root(app, Some(view.into()), window, cx)
    });
}

const FODT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>Hello 世界</text:p><text:p>Second line</text:p></office:text></office:body>
</office:document>"#;

struct Probe {
    view: Entity<LokView>,
    worker: Arc<Mutex<Worker>>,
    doc: Doc,
    dir: PathBuf,
}

fn key(text: &str, cx: &App) -> Keystroke {
    Keystroke::parse_with_os(text, cx.platform_info().os).expect("a keystroke")
}

fn typed(text: &str, cx: &App) -> Keystroke {
    Keystroke {
        key_char: Some(text.into()),
        ..key(text, cx)
    }
}

fn press(window: &mut Window, stroke: Keystroke, cx: &mut App) {
    window.dispatch_keystroke(stroke.clone(), cx);
    window.dispatch_event(PlatformInput::KeyUp(KeyUpEvent { keystroke: stroke }), cx);
}

/// Holds the window for an outside AX reader, then quits.
fn hold(cx: &mut App) {
    println!("probe: ax hold");
    cx.spawn(async move |cx| {
        cx.background_executor()
            .timer(std::time::Duration::from_secs(8))
            .await;
        cx.update(|cx| cx.quit());
    })
    .detach();
}

/// A left drag from `from` to `to`; equal points click.
fn drag(window: &mut Window, from: Point<Pixels>, to: Point<Pixels>, cx: &mut App) {
    let modifiers = Modifiers::default();
    window.dispatch_event(
        PlatformInput::MouseDown(MouseDownEvent {
            button: MouseButton::Left,
            position: from,
            modifiers,
            click_count: 1,
            first_mouse: false,
        }),
        cx,
    );
    if from != to {
        window.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent {
                position: to,
                pressed_button: Some(MouseButton::Left),
                modifiers,
            }),
            cx,
        );
    }
    window.dispatch_event(
        PlatformInput::MouseUp(MouseUpEvent {
            button: MouseButton::Left,
            position: to,
            modifiers,
            click_count: 1,
        }),
        cx,
    );
}

impl Probe {
    /// One step per settled frame window; `wait` counts frames.
    fn step(self, step: usize, wait: usize, window: &mut Window, cx: &mut App) {
        if wait < 40 {
            window.on_next_frame(move |window, cx| self.step(step, wait + 1, window, cx));
            return;
        }
        let clipboard_on = std::env::var_os("ELY_CLIPBOARD_TEST").is_some();
        let at = |cx: &App, x, y| self.view.read(cx).window_point((x, y));
        match step {
            0 => {
                let focus = self.view.read(cx).focus_handle().clone();
                window.focus(&focus, cx);
                if std::env::var("ELY_PROBE_AX").as_deref() == Ok("initial") {
                    hold(cx);
                    return;
                }
                let mut ime = ElementInputHandler::new(Bounds::default(), self.view.clone());
                ime.replace_and_mark_text_in_range(None, "ni", Some(2..2), window, cx);
                ime.replace_and_mark_text_in_range(Some(0..2), "你好", None, window, cx);
                ime.unmark_text(window, cx);
                ime.replace_and_mark_text_in_range(None, "zz", None, window, cx);
                ime.replace_and_mark_text_in_range(None, "", None, window, cx);
            }
            1 => {
                let end = at(cx, PAST_END, LINE_ONE);
                drag(window, end, end, cx);
                press(window, typed("X", cx), cx);
            }
            2 => {
                let (from, to) = (at(cx, TEXT_LEFT, LINE_TWO), at(cx, PAST_END, LINE_TWO));
                drag(window, from, to, cx);
                press(window, typed("Z", cx), cx);
                println!(
                    "probe: focused={}",
                    self.view.read(cx).focus_handle().is_focused(window)
                );
            }
            3 if clipboard_on => {
                press(window, key("secondary-a", cx), cx);
                press(window, key("secondary-c", cx), cx);
            }
            4 if clipboard_on => {
                let copied = clipboard::read().expect("the clipboard reads");
                println!("probe: copied={copied:?}");
                press(window, key("secondary-end", cx), cx);
                press(window, key("enter", cx), cx);
                press(window, key("secondary-v", cx), cx);
            }
            3 | 4 => {}
            _ => {
                let out = self.dir.join("out.fodt");
                let mut worker = self.worker.lock().expect("the worker lock");
                self.doc
                    .save_as(&mut worker, &format!("file://{}", out.display()), "fodt")
                    .expect("the save works");
                let xml = std::fs::read_to_string(&out).expect("the save reads");
                println!("probe: paragraphs={}", paragraphs(&xml).join("|"));
                drop(worker);
                if std::env::var("ELY_PROBE_AX").as_deref() == Ok("edited") {
                    hold(cx);
                    return;
                }
                cx.quit();
                return;
            }
        }
        window.on_next_frame(move |window, cx| self.step(step + 1, 0, window, cx));
    }
}

/// Each paragraph's text, tags stripped.
fn paragraphs(xml: &str) -> Vec<String> {
    xml.split("<text:p")
        .skip(1)
        .map(|rest| {
            let body = &rest[rest.find('>').map_or(0, |at| at + 1)..];
            let body = &body[..body.find("</text:p>").unwrap_or(0)];
            let mut text = String::new();
            let mut tag = false;
            for c in body.chars() {
                match c {
                    '<' => tag = true,
                    '>' => tag = false,
                    c if !tag => text.push(c),
                    _ => {}
                }
            }
            text
        })
        .collect()
}
