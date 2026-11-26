use std::{
    io::{BufRead as _, BufReader, Write as _},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

use base64::Engine as _;
use gpui::{Entity, TestAppContext, VisualTestContext};

use super::{SETTLE, addresses, picture};
use crate::{
    engine::{Text, mark},
    text::Source,
    view::MarkView,
};

/// One transparent pixel.
const PIXEL: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

/// PNGs at /a.png and /b.png, a page elsewhere; counted.
fn server() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let port = listener.local_addr().expect("an address").port();
    let served = Arc::new(AtomicUsize::new(0));
    let count = served.clone();
    let png = base64::engine::general_purpose::STANDARD.decode(PIXEL).expect("a png");
    thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = stream.expect("a request");
            let mut reader = BufReader::new(stream.try_clone().expect("a stream"));
            let (mut first, mut line) = (String::new(), String::new());
            reader.read_line(&mut first).expect("a request line");
            while reader.read_line(&mut line).expect("a header") > 2 {
                line.clear();
            }
            let picture = first.contains("/a.png ") || first.contains("/b.png ");
            let (kind, body) = match picture {
                true => ("image/png", png.clone()),
                false => ("text/html", b"<p>no</p>".to_vec()),
            };
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let mut stream = stream;
            stream.write_all(head.as_bytes()).expect("a head");
            stream.write_all(&body).expect("a body");
            count.fetch_add(1, Ordering::SeqCst);
        }
    });
    (format!("http://127.0.0.1:{port}"), served)
}

fn open<'a>(text: &str, cx: &'a mut TestAppContext) -> (Entity<MarkView>, &'a mut VisualTestContext) {
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

fn held(view: &Entity<MarkView>, url: &str, cx: &mut VisualTestContext) -> Option<Result<(), String>> {
    view.read_with(cx, |view, _| {
        let got = view.web.held.get(url)?;
        Some(got.as_ref().map(|_| ()).map_err(|why| why.to_string()))
    })
}

#[test]
fn addresses_are_the_web_pictures_each_once() {
    let text = "![a](https://e.com/a.png) ![b](HTTP://e.com/b.png)\n\n\
        ![again](https://e.com/a.png) ![here](pic.png) ![file](file:///c/p.png)\n\n\
        [a link](https://e.com/page) ![ref][logo]\n\n\
        | t |\n|---|\n| ![cell](https://e.com/c.png) |\n\n\
        > ![quoted](http://e.com/q.png)\n\n\
        [logo]: https://e.com/logo.png\n[logo]: https://e.com/second.png\n";
    assert_eq!(
        addresses(text),
        [
            "https://e.com/a.png",
            "HTTP://e.com/b.png",
            "https://e.com/c.png",
            "http://e.com/q.png",
            "https://e.com/logo.png",
        ]
    );
}

#[test]
fn only_pictures_gpui_draws_are_held_and_svg_is_not() {
    assert!(picture("image/png", vec![1]).is_ok());
    assert!(picture("image/jpg", vec![1]).is_ok(), "the common misspelling");
    assert!(picture("image/svg+xml", vec![1]).is_err(), "SVG may load files");
    assert!(picture("image/x-unknown", vec![1]).is_err());
    assert!(picture("image/png", Vec::new()).is_err());
}

#[gpui::test]
fn web_pictures_wait_until_asked_then_draw(cx: &mut TestAppContext) {
    let (base, served) = server();
    let url = format!("{base}/a.png");
    let (view, cx) = open(&format!("![a]({url})\n"), cx);
    assert_eq!(served.load(Ordering::SeqCst), 0, "nothing fetched unasked");
    assert_eq!(held(&view, &url, cx), None);
    view.update(cx, |view, cx| view.load_web(true, cx));
    cx.run_until_parked();
    assert_eq!(held(&view, &url, cx), Some(Ok(())));
    assert_eq!(served.load(Ordering::SeqCst), 1);
    let mut drawn = None;
    for _ in 0..50 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        drawn = cx.debug_bounds("markdown-picture");
        if drawn.is_some() {
            break;
        }
    }
    assert!(drawn.is_some(), "the fetched picture draws");
    view.update(cx, |view, cx| view.load_web(true, cx));
    cx.run_until_parked();
    assert_eq!(served.load(Ordering::SeqCst), 1, "a held picture is not fetched again");
}

#[gpui::test]
fn a_failure_shows_why_and_only_asking_retries(cx: &mut TestAppContext) {
    let (base, served) = server();
    let url = format!("{base}/page");
    let (view, cx) = open(&format!("![a]({url})\n"), cx);
    view.update(cx, |view, cx| view.set_web_auto(true, cx));
    cx.run_until_parked();
    let why = held(&view, &url, cx).expect("tried").expect_err("a page is no picture");
    assert!(why.starts_with("Could not load image: ") && why.contains("text/html"), "{why}");
    view.update(cx, |view, cx| view.web_changed(cx));
    cx.executor().advance_clock(SETTLE);
    cx.run_until_parked();
    assert_eq!(served.load(Ordering::SeqCst), 1, "typing does not retry a failure");
    view.update(cx, |view, cx| view.load_web(true, cx));
    cx.run_until_parked();
    assert_eq!(served.load(Ordering::SeqCst), 2, "asking does");
}

#[gpui::test]
fn the_setting_loads_pictures_typed_later(cx: &mut TestAppContext) {
    let (base, served) = server();
    let (view, cx) = open("text\n", cx);
    view.update(cx, |view, cx| view.set_web_auto(true, cx));
    cx.run_until_parked();
    let url = format!("{base}/b.png");
    let document = view.read_with(cx, |view, _| view.document().clone());
    document.update(cx, |document, cx| document.set_text(format!("text ![b]({url})\n"), cx));
    cx.run_until_parked();
    assert_eq!(held(&view, &url, cx), None, "typing settles first");
    cx.executor().advance_clock(SETTLE);
    cx.run_until_parked();
    assert_eq!(held(&view, &url, cx), Some(Ok(())));
    assert_eq!(served.load(Ordering::SeqCst), 1);
    view.update(cx, |view, cx| view.set_web_auto(false, cx));
    let later = format!("{base}/a.png");
    document.update(cx, |document, cx| document.set_text(format!("![a]({later})\n"), cx));
    cx.executor().advance_clock(SETTLE);
    cx.run_until_parked();
    assert_eq!(held(&view, &later, cx), None, "off again, nothing more loads");
}
