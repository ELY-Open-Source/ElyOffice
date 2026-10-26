//! Files handed to the app: arguments, Finder, second copies.

use std::{io, path::PathBuf, time::Duration};

use ely_platform::instance::Door;
use futures::{StreamExt, channel::mpsc};
use gpui::{App, Application, Global};

/// How long a hand-over waits for the window.
const TAKING: Duration = Duration::from_secs(5);
/// How long a request waits for the app's answer.
const ANSWERING: Duration = Duration::from_secs(80);

struct Asked(Option<mpsc::UnboundedReceiver<PathBuf>>);

impl Global for Asked {}

/// A second copy's files; `taken` answers once they are queued.
pub(crate) struct Hand {
    files: Vec<PathBuf>,
    taken: std::sync::mpsc::Sender<()>,
}

/// Another program's request; the answer goes back once.
pub struct Ask {
    pub request: String,
    answer: std::sync::mpsc::Sender<Result<String, String>>,
}

impl Ask {
    pub fn answer(self, answer: Result<String, String>) {
        self.answer.send(answer).ok();
    }
}

/// How the app answers other programs.
type Answer = dyn Fn(Ask, &mut App);

/// The app's answers to other programs; none refuses them.
struct Answers(std::rc::Rc<Answer>);
impl Global for Answers {}

/// What the system and other copies hand the running app.
pub(crate) struct Handed {
    asked: mpsc::UnboundedReceiver<PathBuf>,
    hands: Hands,
}

/// Second copies' hand-overs, for the main window.
pub(crate) struct Hands {
    hands: mpsc::UnboundedReceiver<Hand>,
    queue: mpsc::UnboundedSender<PathBuf>,
    asks: mpsc::UnboundedReceiver<Ask>,
}

/// Launch arguments, relative to where the app was started.
pub(crate) fn launch_args() -> io::Result<Vec<PathBuf>> {
    std::env::args_os().skip(1).map(std::path::absolute).collect()
}

/// Queues `files`, then files the system or copies ask for.
pub(crate) fn listen(application: &Application, files: Vec<PathBuf>, door: Door) -> io::Result<Handed> {
    let (queue, asked) = mpsc::unbounded();
    for path in files {
        queue.unbounded_send(path).ok();
    }
    let (sender, hands) = mpsc::unbounded();
    let (asker, asks) = mpsc::unbounded();
    let deliver = move |files| {
        let (taken, took) = std::sync::mpsc::channel();
        sender.unbounded_send(Hand { files, taken }).is_ok() && took.recv_timeout(TAKING).is_ok()
    };
    door.serve(deliver, move |request| {
        let (answer, answered) = std::sync::mpsc::channel();
        asker.unbounded_send(Ask { request, answer }).map_err(|_| "the app is quitting".to_string())?;
        answered.recv_timeout(ANSWERING).map_err(|_| "the app gave no answer".to_string())?
    })?;
    let system = queue.clone();
    application.on_open_urls(move |urls| {
        for url in urls {
            match file_path(&url) {
                Some(path) => {
                    log::info!("run: the system asks to open a file");
                    system.unbounded_send(path).ok();
                }
                None => log::warn!("run: not a file URL, ignored"),
            }
        }
    });
    Ok(Handed { asked, hands: Hands { hands, queue, asks } })
}

/// Keeps the files for `on_open`; the hand-overs go to `home`.
pub(crate) fn keep(handed: Handed, cx: &mut App) -> Hands {
    cx.set_global(Asked(Some(handed.asked)));
    handed.hands
}

/// No files asked at launch, for tests.
#[cfg(any(test, feature = "test-support"))]
pub fn install_no_launch_files(cx: &mut App) {
    cx.set_global(Asked(Some(mpsc::unbounded().1)));
}

/// Files asked before the app started: arguments and Finder's.
pub fn launch_files(cx: &mut App) -> Vec<PathBuf> {
    let asked = cx.global_mut::<Asked>().0.as_mut();
    let asked = asked.expect("launch_files comes before on_open");
    std::iter::from_fn(|| asked.try_recv().ok()).collect()
}

/// Hands each file asked from now on to `open`.
pub fn on_open(cx: &mut App, mut open: impl FnMut(PathBuf, &mut App) + 'static) {
    let mut asked = cx
        .global_mut::<Asked>()
        .0
        .take()
        .expect("on_open runs once per app");
    cx.spawn(async move |cx| {
        while let Some(path) = asked.next().await {
            cx.update(|cx| open(path, cx));
        }
    })
    .detach();
}

/// Answers other programs' requests from now on.
pub fn on_ask(cx: &mut App, answer: impl Fn(Ask, &mut App) + 'static) {
    cx.set_global(Answers(std::rc::Rc::new(answer)));
}

/// Each hand-over: the main window forward, then its files queued.
pub(crate) fn home(app: ely_platform::AppId, hands: Hands, cx: &mut App) {
    let Hands { mut hands, queue, mut asks } = hands;
    cx.spawn(async move |cx| {
        while let Some(ask) = asks.next().await {
            cx.update(|cx| match cx.try_global::<Answers>().map(|answers| answers.0.clone()) {
                Some(answer) => answer(ask, cx),
                None => ask.answer(Err(format!("{} takes no requests", app.name()))),
            });
        }
    })
    .detach();
    cx.spawn(async move |cx| {
        while let Some(hand) = hands.next().await {
            // No answer: the other copy says the files went untaken.
            if cx.update(crate::tray::show_main).is_none() {
                log::warn!("run: the main window is gone, so a hand-over is refused");
                continue;
            }
            for path in hand.files {
                queue.unbounded_send(path).ok();
            }
            hand.taken.send(()).ok();
        }
    })
    .detach();
}

/// A `file://` URL's path, percent escapes decoded.
fn file_path(url: &str) -> Option<PathBuf> {
    let rest = url.strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    if !rest.starts_with('/') {
        return None;
    }
    let mut bytes = Vec::with_capacity(rest.len());
    let mut raw = rest.bytes();
    while let Some(byte) = raw.next() {
        if byte != b'%' {
            bytes.push(byte);
            continue;
        }
        let hex = [raw.next()?, raw.next()?];
        bytes.push(u8::from_str_radix(std::str::from_utf8(&hex).ok()?, 16).ok()?);
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Some(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
    }
    #[cfg(not(unix))]
    {
        // `file:///C:/a` holds `/C:/a`.
        let text = String::from_utf8(bytes).ok()?;
        Some(PathBuf::from(text.strip_prefix('/').unwrap_or(&text)))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::file_path;
    use std::path::PathBuf;

    #[test]
    fn file_urls_decode_and_others_do_not() {
        assert_eq!(file_path("file:///Users/a/b%20c%23.md"), Some(PathBuf::from("/Users/a/b c#.md")));
        assert_eq!(file_path("file://localhost/tmp/x"), Some(PathBuf::from("/tmp/x")));
        assert_eq!(file_path("file:///a/%E4%BD%A0.md"), Some(PathBuf::from("/a/你.md")));
        assert_eq!(file_path("https://example.com/a"), None);
        assert_eq!(file_path("file:///a/%zz"), None);
        assert_eq!(file_path("file:///a/%4"), None);
    }
}

#[cfg(test)]
mod home_tests {
    use super::*;
    use gpui::AnyWindowHandle;
    use gpui::{Context, IntoElement, Render, TestAppContext, Window};

    struct Blank;

    impl Render for Blank {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            gpui::div()
        }
    }

    /// `home` on `main`; returns its hand-over, file and request queues.
    fn homed(
        main: AnyWindowHandle,
        cx: &mut TestAppContext,
    ) -> (mpsc::UnboundedSender<Hand>, mpsc::UnboundedReceiver<PathBuf>, mpsc::UnboundedSender<Ask>) {
        let (sender, hands) = mpsc::unbounded();
        let (queue, asked) = mpsc::unbounded();
        let (asker, asks) = mpsc::unbounded();
        cx.update(|cx| {
            crate::tray::set_main(ely_platform::AppId::Mark, main, cx);
            home(ely_platform::AppId::Mark, Hands { hands, queue, asks }, cx)
        });
        (sender, asked, asker)
    }

    fn ask(asker: &mpsc::UnboundedSender<Ask>, request: &str) -> std::sync::mpsc::Receiver<Result<String, String>> {
        let (answer, answered) = std::sync::mpsc::channel();
        asker.unbounded_send(Ask { request: request.into(), answer }).expect("sent");
        answered
    }

    #[gpui::test]
    fn a_request_goes_to_the_app_or_is_refused(cx: &mut TestAppContext) {
        let main: AnyWindowHandle = cx.add_window(|_, _| Blank).into();
        let (_, _, asker) = homed(main, cx);
        let refused = ask(&asker, "docs");
        cx.run_until_parked();
        assert_eq!(refused.try_recv(), Ok(Err("ElyMark takes no requests".into())));
        cx.update(|cx| on_ask(cx, |ask, _| {
            let echo = ask.request.to_uppercase();
            ask.answer(Ok(echo))
        }));
        let answered = ask(&asker, "docs");
        cx.run_until_parked();
        assert_eq!(answered.try_recv(), Ok(Ok("DOCS".into())));
    }

    fn handed(sender: &mpsc::UnboundedSender<Hand>) -> std::sync::mpsc::Receiver<()> {
        let (taken, took) = std::sync::mpsc::channel();
        sender.unbounded_send(Hand { files: vec![PathBuf::from("x.md")], taken }).expect("sent");
        took
    }

    #[gpui::test]
    fn a_hand_over_raises_the_main_window_then_queues(cx: &mut TestAppContext) {
        let main: AnyWindowHandle = cx.add_window(|_, _| Blank).into();
        let _settings = cx.add_window(|_, _| Blank);
        let (sender, mut asked, _) = homed(main, cx);
        assert_ne!(cx.update(|cx| cx.active_window()), Some(main), "another window is in front");
        let took = handed(&sender);
        cx.run_until_parked();
        assert_eq!(cx.update(|cx| cx.active_window()), Some(main));
        assert_eq!(asked.try_recv().ok(), Some(PathBuf::from("x.md")));
        assert!(took.try_recv().is_ok(), "the other copy hears it was taken");
    }

    #[gpui::test]
    fn a_hand_over_without_the_main_window_is_refused(cx: &mut TestAppContext) {
        let main: AnyWindowHandle = cx.add_window(|_, _| Blank).into();
        let _settings = cx.add_window(|_, _| Blank);
        let (sender, mut asked, _) = homed(main, cx);
        main.update(cx, |_, window, _| window.remove_window()).expect("closed");
        cx.run_until_parked();
        let took = handed(&sender);
        cx.run_until_parked();
        assert!(took.try_recv().is_err(), "no answer, so the other copy reports it");
        assert!(asked.try_recv().is_err(), "nothing queued for a gone window");
    }
}
