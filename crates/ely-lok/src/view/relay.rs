//! Kernel dialogs Ely closed, clicked links, dropped files.

use std::path::{Path, PathBuf};

use gpui::{Context, EventEmitter, ExternalPaths, Modifiers, Window};

use super::LokView;
use super::input::Input;

/// A closed kernel dialog: its title and text.
pub struct Closed {
    pub title: String,
    pub body: String,
}

/// A link the user clicked, as the kernel names it.
pub struct Linked(pub String);

/// Dropped files that are no pictures; the app opens them.
pub struct Dropped(pub Vec<PathBuf>);

impl EventEmitter<Closed> for LokView {}

impl EventEmitter<Linked> for LokView {}

impl EventEmitter<Dropped> for LokView {}

/// Picture types the kernel reads.
const PICTURES: [&str; 9] = ["png", "jpg", "jpeg", "gif", "bmp", "webp", "svg", "tif", "tiff"];
const ESCAPE: i32 = 1281;

fn picture(path: &Path) -> bool {
    let ext = path.extension().and_then(|ext| ext.to_str()).map(str::to_ascii_lowercase);
    ext.is_some_and(|ext| PICTURES.contains(&ext.as_str()))
}

impl LokView {
    pub(super) fn relay(&mut self, blocked: Vec<(String, String)>, links: Vec<String>, cx: &mut Context<Self>) {
        for (title, body) in blocked {
            cx.emit(Closed { title, body });
        }
        for link in links {
            cx.emit(Linked(link));
        }
    }

    /// Pictures go in where they fall; other files go on.
    pub(super) fn on_drop(&mut self, dropped: &ExternalPaths, window: &mut Window, cx: &mut Context<Self>) {
        let (pictures, rest): (Vec<PathBuf>, Vec<PathBuf>) = dropped.paths().iter().cloned().partition(|path| picture(path));
        if !rest.is_empty() {
            cx.emit(Dropped(rest));
        }
        if pictures.is_empty() {
            return;
        }
        log::info!("lok: {} dropped pictures go in", pictures.len());
        window.focus(&self.focus, cx);
        // A click puts the cursor where they fell.
        let at = window.mouse_position();
        for kind in [0, 1] {
            self.mouse(kind, at, 1, 1, &Modifiers::default());
        }
        for path in pictures {
            // A selected picture would be replaced, not joined.
            self.send(Input::Key { char_code: 0, key_code: ESCAPE });
            if let Err(error) = self.run("image.insert", &[&crate::file_url(&path)]) {
                cx.emit(super::LokError(format!("a dropped picture did not go in: {error}")));
            }
        }
    }
}
