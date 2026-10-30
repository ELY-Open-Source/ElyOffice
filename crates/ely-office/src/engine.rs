//! What a document engine gives the shell (§6).

use std::{
    io,
    path::Path,
    sync::{Arc, Mutex},
};

use ely_platform::AppId;
use gpui::{App, Context, Entity, FocusHandle, Window};

use crate::Host;

/// Ends a stuck worker; runs outside its lock.
pub type Stop = Arc<dyn Fn() -> io::Result<()> + Send + Sync>;

/// Where a load starts.
pub enum Load<'a, I> {
    /// A file and its password, empty for none.
    File(&'a Path, &'a str),
    /// Text bytes already at the path, read as told.
    Import(&'a Path, &'a I),
}

/// A kernel behind the shell: loads, writes, history, views.
pub trait Engine: Sized + 'static {
    type Worker: Send + 'static;
    type Doc: Clone + Send + 'static;
    type View: 'static;
    /// What a load hands the window.
    type Loaded: Send + 'static;
    /// How text files are read.
    type Import: Clone + Send + 'static;

    /// Loads off the window; a locked file fails PermissionDenied.
    fn load(worker: &mut Self::Worker, from: Load<Self::Import>) -> io::Result<Self::Loaded>;
    /// The view over a load, its document and history mark.
    fn open(
        worker: &Arc<Mutex<Self::Worker>>,
        loaded: Self::Loaded,
        window: &mut Window,
        cx: &mut App,
    ) -> io::Result<(Self::Doc, Entity<Self::View>, String)>;
    /// Checked before the file is overwritten; a risk stops it.
    fn unsound(_worker: &mut Self::Worker, _doc: &Self::Doc) -> io::Result<Option<crate::Risk>> {
        Ok(None)
    }
    /// What saving in place would lose, macros aside.
    fn risk(_loaded: &Self::Loaded) -> Option<crate::Risk> {
        None
    }
    /// The undo history as a mark to compare.
    fn history(worker: &mut Self::Worker, doc: &Self::Doc) -> io::Result<String>;
    /// Writes `doc` as `format` to `path`.
    fn write(
        worker: &mut Self::Worker,
        doc: &Self::Doc,
        path: &Path,
        format: &str,
    ) -> io::Result<()>;
    /// A copy of `doc` at `path`; `doc` stays.
    fn snapshot(_worker: &mut Self::Worker, _doc: &Self::Doc, _path: &Path, format: &str) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::Unsupported, format!("no snapshot as .{format}")))
    }
    fn close(worker: &mut Self::Worker, doc: Self::Doc) -> io::Result<()>;
    /// A closing view's images, for the shell to free.
    fn retire(view: &Entity<Self::View>, cx: &mut App) -> ely_gpui_component::primitives::Retired;
    /// Why the worker ended, if it did (§6.3).
    fn ended(_worker: &mut Self::Worker) -> Option<String> {
        None
    }
    /// Ends the worker from outside its lock; None cannot.
    fn stopper(_worker: &Self::Worker) -> Option<Stop> {
        None
    }
    /// A fresh worker in place of an ended one.
    fn restart(_app: AppId, _worker: &mut Self::Worker) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::Unsupported, "this engine does not restart"))
    }
    /// Every input sent has reached the kernel.
    fn settled(view: &Self::View) -> bool;
    /// Edits landed so far.
    fn edited(view: &Self::View) -> u64;
    /// An edit is sent but not landed.
    fn edit_pending(view: &Self::View) -> bool;
    fn focus(view: &Self::View) -> FocusHandle;
    /// The document was saved at history `mark`.
    fn saved(view: &Entity<Self::View>, mark: &str, cx: &mut App);
    /// Calls `reported` with the modified flag, `failed` with errors.
    fn watch<H: Host<Engine = Self>>(
        view: &Entity<Self::View>,
        cx: &mut Context<H>,
        reported: impl Fn(&mut H, &Entity<Self::View>, bool, &mut Context<H>) + 'static,
        failed: impl Fn(&mut H, &str, &mut Context<H>) + 'static,
    );
}
