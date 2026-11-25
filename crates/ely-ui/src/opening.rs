//! Files handed to the app: arguments, Finder, second copies.

use std::{
    ffi::OsString,
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use ely_platform::{AppId, instance::{Door, Launch}};
use futures::{StreamExt, channel::mpsc};
use gpui::{AnyWindowHandle, App, Application, Global};

/// How long a hand-over waits for the window.
const TAKING: Duration = Duration::from_secs(5);
/// How long a request waits for the app's answer.
const ANSWERING: Duration = Duration::from_secs(80);

/// A file to open, and the window it belongs to.
pub type Opened = (PathBuf, Option<AnyWindowHandle>);

struct Asked(Option<mpsc::UnboundedReceiver<Opened>>);

impl Global for Asked {}

/// The sending end of what `on_open` reads.
struct Queue(mpsc::UnboundedSender<Opened>);

impl Global for Queue {}

/// Hands `path` to `on_open` for `window`, as a drop does.
pub(crate) fn queue(path: PathBuf, window: Option<AnyWindowHandle>, cx: &App) -> io::Result<()> {
    let queue = cx.try_global::<Queue>().ok_or_else(|| io::Error::other("the app takes no files yet"))?;
    queue.0.unbounded_send((path, window)).map_err(|_| io::Error::other("the app takes no more files"))
}

/// A second copy's ask; `taken` answers once queued.
pub(crate) struct Hand {
    launch: Launch,
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

#[cfg(any(test, feature = "test-support"))]
impl Ask {
    /// A request and where its answer lands, for tests.
    pub fn for_tests(request: &str) -> (Self, std::sync::mpsc::Receiver<Result<String, String>>) {
        let (answer, answered) = std::sync::mpsc::channel();
        (Self { request: request.into(), answer }, answered)
    }
}

/// How the app answers other programs.
type Answer = dyn Fn(Ask, &mut App);

/// The app's answers to other programs; none refuses them.
struct Answers(std::rc::Rc<Answer>);
impl Global for Answers {}

/// What the system and other copies hand the running app.
pub(crate) struct Handed {
    asked: mpsc::UnboundedReceiver<Opened>,
    hands: Hands,
}

/// Second copies' hand-overs, for the main window.
pub(crate) struct Hands {
    hands: mpsc::UnboundedReceiver<Hand>,
    queue: mpsc::UnboundedSender<Opened>,
    asks: mpsc::UnboundedReceiver<Ask>,
}

/// Launch arguments, relative to where the app was started.
pub(crate) fn launch_args() -> io::Result<Launch> {
    launch_from(std::env::args_os().skip(1))
}

/// Files, and `--new-window` asking for a window of its own.
fn launch_from(args: impl Iterator<Item = OsString>) -> io::Result<Launch> {
    let mut launch = Launch::default();
    for arg in args {
        match arg.to_str() {
            Some(NEW_WINDOW) => launch.new_window = true,
            _ => launch.files.push(launch_arg(arg)?),
        }
    }
    Ok(launch)
}

pub(crate) const NEW_WINDOW: &str = "--new-window";

/// Drops `--new-window` for a one-window app; whether it was set.
pub(crate) fn one_window(launch: &mut Launch) -> bool {
    std::mem::take(&mut launch.new_window)
}

/// A path, a `file://` URL, or a whole mail link.
fn launch_arg(arg: OsString) -> io::Result<PathBuf> {
    match arg.to_str() {
        Some(link) if is_mail_link(link) => Ok(PathBuf::from(link)),
        Some(url) if url.starts_with("file://") => {
            file_path(url).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, format!("an unreadable file URL: {url}")))
        }
        _ => std::path::absolute(arg),
    }
}

fn is_mail_link(text: &str) -> bool {
    text.get(..7).is_some_and(|scheme| scheme.eq_ignore_ascii_case("mailto:"))
}

/// The `mailto:` link an opened item is, if any.
pub fn mail_link(opened: &Path) -> Option<&str> {
    opened.to_str().filter(|text| is_mail_link(text))
}

/// Apps whose files draw late, behind the scene.
pub(crate) fn paged(app: AppId) -> bool {
    AppId::DOCUMENTS.contains(&app) || app == AppId::Pdf
}

/// Queues `files`, then files the system or copies ask for.
pub(crate) fn listen(app: AppId, application: &Application, files: Vec<PathBuf>, door: Door) -> io::Result<Handed> {
    let (queue, asked) = mpsc::unbounded();
    for path in files {
        queue.unbounded_send((path, None)).ok();
    }
    let (sender, hands) = mpsc::unbounded();
    let (asker, asks) = mpsc::unbounded();
    let deliver = move |launch| {
        let (taken, took) = std::sync::mpsc::channel();
        sender.unbounded_send(Hand { launch, taken }).is_ok() && took.recv_timeout(TAKING).is_ok()
    };
    door.serve(deliver, move |request| {
        let (answer, answered) = std::sync::mpsc::channel();
        asker.unbounded_send(Ask { request, answer }).map_err(|_| "the app is quitting".to_string())?;
        answered.recv_timeout(ANSWERING).map_err(|_| "the app gave no answer".to_string())?
    })?;
    let system = queue.clone();
    application.on_open_urls(move |urls| {
        for url in urls {
            if is_mail_link(&url) {
                log::info!("run: the system asks to write to a mail link");
                system.unbounded_send((PathBuf::from(url), None)).ok();
                continue;
            }
            match file_path(&url) {
                Some(path) => {
                    log::info!("run: the system asks to open a file");
                    // macOS hands launch files here.
                    if paged(app) && !ely_platform::startup::has_started() {
                        ely_platform::startup::await_document();
                    }
                    system.unbounded_send((path, None)).ok();
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
    cx.set_global(Queue(handed.hands.queue.clone()));
    handed.hands
}

/// No files asked at launch, for tests.
#[cfg(any(test, feature = "test-support"))]
pub fn install_no_launch_files(cx: &mut App) {
    let (queue, asked) = mpsc::unbounded();
    cx.set_global(Asked(Some(asked)));
    cx.set_global(Queue(queue));
}

/// Files asked before the app started: arguments and Finder's.
pub fn launch_files(cx: &mut App) -> Vec<PathBuf> {
    let asked = cx.global_mut::<Asked>().0.as_mut();
    let asked = asked.expect("launch_files comes before on_open");
    std::iter::from_fn(|| asked.try_recv().ok()).map(|(path, _)| path).collect()
}

/// Hands each later file, and its window, to `open`.
pub fn on_open(cx: &mut App, mut open: impl FnMut(PathBuf, Option<AnyWindowHandle>, &mut App) + 'static) {
    let mut asked = cx
        .global_mut::<Asked>()
        .0
        .take()
        .expect("on_open runs once per app");
    cx.spawn(async move |cx| {
        while let Some((path, window)) = asked.next().await {
            cx.update(|cx| open(path, window, cx));
        }
    })
    .detach();
}

/// Answers other programs' requests from now on.
pub fn on_ask(cx: &mut App, answer: impl Fn(Ask, &mut App) + 'static) {
    cx.set_global(Answers(std::rc::Rc::new(answer)));
}

/// Hands `ask` to the answers, as when it arrived.
pub fn ask_again(app: ely_platform::AppId, ask: Ask, cx: &mut App) {
    match cx.try_global::<Answers>().map(|answers| answers.0.clone()) {
        Some(answer) => answer(ask, cx),
        None => ask.answer(Err(format!("{} takes no requests", app.name()))),
    }
}

/// Each hand-over: the main window forward, then its files queued.
pub(crate) fn home(app: ely_platform::AppId, hands: Hands, cx: &mut App) {
    let Hands { mut hands, queue, mut asks } = hands;
    cx.spawn(async move |cx| {
        while let Some(ask) = asks.next().await {
            cx.update(|cx| ask_again(app, ask, cx));
        }
    })
    .detach();
    cx.spawn(async move |cx| {
        while let Some(hand) = hands.next().await {
            // No answer: the other copy says the files went untaken.
            let shown = cx.update(|cx| match hand.launch.new_window {
                true => crate::windows::open(hand.launch.files.is_empty(), cx).inspect_err(|error| log::error!("run: a new window did not open: {error:#}")).ok(),
                false => crate::tray::show_main(cx),
            });
            let Some(shown) = shown else {
                log::warn!("run: no window takes the hand-over, so it is refused");
                continue;
            };
            // Each file keeps the window chosen now.
            for path in hand.launch.files {
                queue.unbounded_send((path, Some(shown))).ok();
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
mod launch_tests {
    use super::{launch_arg, launch_from, mail_link, one_window};
    use std::path::{Path, PathBuf};

    #[test]
    fn a_one_window_app_keeps_the_files_and_drops_the_flag() {
        let mut launch = launch_from(["a.md", "--new-window"].into_iter().map(Into::into)).expect("read");
        assert!(one_window(&mut launch), "the flag was there");
        assert!(!launch.new_window, "so a hand-over asks for no new window");
        assert!(launch.files.len() == 1 && launch.files[0].ends_with("a.md"), "the file stays");
        assert!(!one_window(&mut launch), "and is gone");
    }

    #[test]
    fn new_window_is_a_flag_not_a_file() {
        let launch = launch_from(["a.xlsx", "--new-window"].into_iter().map(Into::into)).expect("read");
        assert!(launch.new_window);
        assert_eq!(launch.files.len(), 1);
        assert!(launch.files[0].ends_with("a.xlsx"));
        assert!(!launch_from(["b.pdf"].into_iter().map(Into::into)).expect("read").new_window);
    }

    #[test]
    fn mail_links_stay_whole_and_file_urls_become_paths() {
        let link = "mailto:a@b.c?subject=x%20y";
        assert_eq!(launch_arg(link.into()).expect("a link"), PathBuf::from(link));
        assert_eq!(mail_link(Path::new("MAILTO:a@b.c")), Some("MAILTO:a@b.c"));
        assert_eq!(mail_link(Path::new("mailto.md")), None);
        #[cfg(windows)]
        let (url, path) = ("file:///C:/a%20b.eml", "C:/a b.eml");
        #[cfg(unix)]
        let (url, path) = ("file:///tmp/a%20b.eml", "/tmp/a b.eml");
        assert_eq!(launch_arg(url.into()).expect("a file URL"), PathBuf::from(path));
        assert!(launch_arg("file:///a/%zz".into()).is_err(), "a broken URL is refused");
        assert!(launch_arg("a.eml".into()).expect("a path").is_absolute());
    }
}

#[cfg(test)]
mod home_tests {
    use super::*;
    use gpui::AnyWindowHandle;
    use gpui::{AppContext as _, Context, IntoElement, Render, TestAppContext, Window};

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
    ) -> (mpsc::UnboundedSender<Hand>, mpsc::UnboundedReceiver<Opened>, mpsc::UnboundedSender<Ask>) {
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
        let launch = Launch { files: vec![PathBuf::from("x.md")], new_window: false };
        sender.unbounded_send(Hand { launch, taken }).expect("sent");
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
        assert_eq!(asked.try_recv().ok(), Some((PathBuf::from("x.md"), Some(main))));
        assert!(took.try_recv().is_ok(), "the other copy hears it was taken");
    }

    #[gpui::test]
    fn a_new_window_hand_over_opens_one_then_queues(cx: &mut TestAppContext) {
        cx.update(|cx| {
            ely_gpui_component::init_for_tests(cx);
            crate::settings::install_defaults(cx);
            crate::theme::install(cx);
            crate::i18n::install_as("en-US", cx);
        });
        let more = |_: &mut App, _: bool| -> io::Result<crate::windows::Build> {
            Ok(Box::new(|window, cx| {
                let blank = cx.new(|_| Blank);
                crate::tool_root(ely_platform::AppId::Mark, blank.into(), Vec::new(), window, cx)
            }))
        };
        cx.update(|cx| crate::windows::allow(ely_platform::AppId::Mark, more, cx));
        let main: AnyWindowHandle = cx.add_window(|_, _| Blank).into();
        let (sender, mut asked, _) = homed(main, cx);
        let (taken, took) = std::sync::mpsc::channel();
        let launch = Launch { files: vec![PathBuf::from("x.md")], new_window: true };
        sender.unbounded_send(Hand { launch, taken }).expect("sent");
        cx.run_until_parked();
        assert_eq!(cx.update(|cx| cx.windows().len()), 2, "a window of its own");
        let first = cx.update(|cx| crate::tray::main(cx)).expect("the new window");
        assert_ne!(first, main, "the new window is in front");
        // A second ask before the first file opens.
        let (taken, took_too) = std::sync::mpsc::channel();
        let launch = Launch { files: vec![PathBuf::from("y.md")], new_window: true };
        sender.unbounded_send(Hand { launch, taken }).expect("sent");
        cx.run_until_parked();
        let second = cx.update(|cx| crate::tray::main(cx)).expect("another window");
        assert_ne!(second, first);
        assert_eq!(asked.try_recv().ok(), Some((PathBuf::from("x.md"), Some(first))), "each file keeps its window");
        assert_eq!(asked.try_recv().ok(), Some((PathBuf::from("y.md"), Some(second))));
        assert!(took.try_recv().is_ok() && took_too.try_recv().is_ok());
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
