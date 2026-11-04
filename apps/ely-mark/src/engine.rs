//! Markdown files as the shell's engine (§14.3).

use std::{
    convert::Infallible,
    hash::{DefaultHasher, Hash as _, Hasher as _},
    io,
    path::Path,
    sync::{Arc, Mutex},
};

use ely_office::{Engine, Host, Load, Tabs as _};
use gpui::{App, AppContext as _, Context, Entity, FocusHandle, Window};

use crate::text::Source;
use crate::view::{Failed, MarkView, Refused, Reported};

/// A file as read, and its text now.
pub struct Text {
    pub source: Source,
    pub current: String,
    /// The last write's mark, until history reads it.
    written: Option<String>,
}

impl Text {
    /// A file as read, not yet edited.
    pub fn new(source: Source) -> Text {
        let current = source.text.clone();
        Text {
            source,
            current,
            written: None,
        }
    }
}

pub type Doc = Arc<Mutex<Text>>;

/// The history mark: this text's length and hash.
pub fn mark(text: &str) -> String {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    format!("{}:{:016x}", text.len(), hasher.finish())
}

pub struct Mark;

impl Engine for Mark {
    /// Files read and written in place: no worker.
    type Worker = ();
    type Doc = Doc;
    type View = MarkView;
    type Loaded = Source;
    type Import = Infallible;

    fn load(_: &mut (), from: Load<Infallible>) -> io::Result<Source> {
        let path = match from {
            Load::File(path, _) => path,
            Load::Import(_, never) => match *never {},
        };
        // The shell reads PermissionDenied as a locked file.
        let bytes = std::fs::read(path).map_err(|error| match error.kind() {
            io::ErrorKind::PermissionDenied => io::Error::other(error),
            _ => error,
        })?;
        Source::decode(bytes)
    }

    fn open(
        _: &Arc<Mutex<()>>,
        loaded: Source,
        window: &mut Window,
        cx: &mut App,
    ) -> io::Result<(Doc, Entity<MarkView>, String)> {
        let mark = mark(&loaded.text);
        let doc = Arc::new(Mutex::new(Text::new(loaded)));
        let saved = mark.clone();
        let view = cx.new(|cx| MarkView::new(Arc::clone(&doc), saved, window, cx));
        Ok((doc, view, mark))
    }

    /// After a write, what it wrote: later edits stay unsaved.
    fn history(_: &mut (), doc: &Doc) -> io::Result<String> {
        let mut text = doc.lock().expect("the text lock");
        Ok(text.written.take().unwrap_or_else(|| mark(&text.current)))
    }

    fn write(_: &mut (), doc: &Doc, path: &Path, format: &str, _: bool) -> io::Result<()> {
        if !matches!(format, "md" | "markdown") {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("writing .{format}"),
            ));
        }
        // The window reads the text; the disk waits unlocked.
        let (bytes, written) = {
            let text = doc.lock().expect("the text lock");
            (text.source.encode(&text.current), mark(&text.current))
        };
        std::fs::write(path, bytes)?;
        doc.lock().expect("the text lock").written = Some(written);
        Ok(())
    }

    fn close(_: &mut (), _: Doc) -> io::Result<()> {
        Ok(())
    }

    /// Pictures leave with their elements; nothing to hand back.
    fn retire(_: &Entity<MarkView>, _: &mut App) -> ely_gpui_component::primitives::Retired {
        ely_gpui_component::primitives::Retired::default()
    }

    fn settled(view: &MarkView) -> bool {
        view.settled()
    }

    fn edited(view: &MarkView) -> u64 {
        view.changed()
    }

    fn edit_pending(_: &MarkView) -> bool {
        false
    }

    fn focus(view: &MarkView) -> FocusHandle {
        view.focus.clone()
    }

    fn saved(view: &Entity<MarkView>, mark: &str, cx: &mut App) {
        view.update(cx, |view, cx| view.saved(mark, cx));
    }

    fn watch<H: Host<Engine = Self>>(
        view: &Entity<MarkView>,
        cx: &mut Context<H>,
        reported: impl Fn(&mut H, &Entity<MarkView>, bool, &mut Context<H>) + 'static,
        failed: impl Fn(&mut H, &str, &mut Context<H>) + 'static,
    ) {
        cx.subscribe(view, move |host, _, error: &Failed, cx| {
            failed(host, &error.0, cx)
        })
        .detach();
        cx.subscribe(view, |host, _, refused: &Refused, cx| {
            let said = crate::blocks::refusal(refused.0, cx);
            host.toast(said, ely_gpui_component::primitives::Severity::Info, cx)
        })
        .detach();
        cx.subscribe(view, move |host, view, _: &Reported, cx| {
            let modified = view.read(cx).modified();
            reported(host, &view, modified, cx)
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use ely_office::Engine as _;

    use super::{Mark, Text, mark};
    use crate::text::Source;

    #[test]
    fn a_save_marks_the_text_it_wrote() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let source = Source::decode(b"Alpha.\n".to_vec()).expect("reads");
        let doc = Arc::new(Mutex::new(Text::new(source)));
        let path = dir.path().join("save.md");
        Mark::write(&mut (), &doc, &path, "md", false).expect("writes");
        // An edit lands between the write and the history read.
        doc.lock().expect("the text lock").current = "xAlpha.\n".into();
        let saved = Mark::history(&mut (), &doc).expect("a mark");
        assert_eq!(saved, mark("Alpha.\n"));
        assert_eq!(
            Mark::history(&mut (), &doc).expect("a mark"),
            mark("xAlpha.\n")
        );
    }

    /// A pipe in `dir`; its reader opens once written.
    #[cfg(unix)]
    fn held_pipe(dir: &std::path::Path) -> (std::path::PathBuf, impl FnOnce() -> std::fs::File) {
        let path = dir.join("held.md");
        let made = std::process::Command::new("mkfifo").arg(&path).status();
        assert!(made.expect("mkfifo runs").success(), "a pipe");
        let reading = path.clone();
        // A pipe's open waits for its writer.
        (path, move || std::fs::File::open(reading).expect("the reader opens"))
    }

    // Windows writes cannot open a pipe: Unix holds the disk.
    #[cfg(unix)]
    #[test]
    fn a_waiting_disk_leaves_the_text_free() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let (path, reader) = held_pipe(dir.path());
        let text = "x".repeat(1 << 20);
        let doc = Arc::new(Mutex::new(Text::new(Source::decode(Vec::new()).expect("reads"))));
        doc.lock().expect("the text lock").current = text.clone();
        let writing = {
            let doc = Arc::clone(&doc);
            std::thread::spawn(move || Mark::write(&mut (), &doc, &path, "md"))
        };
        let mut reader = reader();
        // The write waits on its reader; the window still reads.
        assert!(doc.try_lock().is_ok(), "the disk holds the text lock");
        let mut got = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut got).expect("reads the write");
        writing.join().expect("the writer ends").expect("writes");
        assert!(got == text.as_bytes(), "the pipe got {} bytes", got.len());
        assert_eq!(Mark::history(&mut (), &doc).expect("a mark"), mark(&text));
    }
}
