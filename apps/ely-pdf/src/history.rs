//! Reading pages stay outside the PDF.

use std::{
    io,
    path::{Path, PathBuf},
};

use ely_gpui_component::primitives::Severity;
use ely_office::{Tab, Tabs as _};
use gpui::Context;

use crate::{engine::Pdf, viewer::Viewer};

type Pages = Vec<(PathBuf, usize)>;

fn read(file: &Path) -> io::Result<Pages> {
    match std::fs::read(file) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

fn page(file: &Path, path: &Path) -> io::Result<Option<usize>> {
    let path = std::fs::canonicalize(path)?;
    Ok(read(file)?
        .into_iter()
        .find(|(known, _)| *known == path)
        .map(|(_, page)| page))
}

fn remember(file: &Path, path: &Path, page: usize) -> io::Result<()> {
    let path = std::fs::canonicalize(path)?;
    let mut pages = read(file)?;
    pages.retain(|(known, _)| *known != path);
    pages.insert(0, (path, page));
    pages.truncate(1000);
    let bytes = serde_json::to_vec(&pages).map_err(io::Error::other)?;
    ely_files::save_atomic(file, &bytes, 0)
        .map(drop)
        .map_err(io::Error::other)
}

impl Viewer {
    pub(crate) fn restore_reading_page(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(tab) = self.tab(id) else { return };
        let Some(path) = tab.session.path() else {
            return;
        };
        let file = self.core.work.with_file_name("reading-pages.json");
        match page(&file, path) {
            Ok(Some(page)) => {
                tab.view.update(cx, |view, cx| {
                    view.restore_page = Some(page.min(view.sizes.len().saturating_sub(1)));
                    cx.notify();
                });
                log::info!("pdf: restores a reading page");
            }
            Ok(None) => {}
            Err(error) => self.reading_failed(&error, cx),
        }
    }

    pub(crate) fn keep_reading_page(&self, tab: &Tab<(), Pdf>, cx: &mut Context<Self>) {
        let Some(path) = tab.session.path() else {
            return;
        };
        let view = tab.view.read(cx);
        let page = view.restore_page.unwrap_or(view.current);
        let file = self.core.work.with_file_name("reading-pages.json");
        match remember(&file, path, page) {
            Ok(()) => log::info!("pdf: remembers a reading page"),
            Err(error) => self.reading_failed(&error, cx),
        }
    }

    fn reading_failed(&self, error: &io::Error, cx: &mut Context<Self>) {
        log::error!("pdf: reading position failed ({:?})", error.kind());
        let text = Self::text(cx, "pdf.reading.failed", &[("error", &error.to_string())]);
        self.toast(text, Severity::Danger, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_survive_reopening_and_belong_to_each_file() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let state = dir.path().join("pages.json");
        let a = dir.path().join("a.pdf");
        let b = dir.path().join("b.pdf");
        std::fs::write(&a, b"a").expect("a file");
        std::fs::write(&b, b"b").expect("a file");
        assert_eq!(page(&state, &a).expect("no history"), None);
        remember(&state, &a, 4).expect("remembers");
        remember(&state, &b, 8).expect("remembers");
        remember(&state, &a, 2).expect("updates");
        assert_eq!(page(&state, &a).expect("reopens"), Some(2));
        assert_eq!(page(&state, &b).expect("reopens"), Some(8));
        assert_eq!(read(&state).expect("history").len(), 2);
        assert_eq!(std::fs::read(&a).expect("unchanged"), b"a");
        std::fs::write(&state, b"broken json").expect("corrupts");
        assert!(page(&state, &a).is_err());
        assert!(remember(&state, &a, 0).is_err());
        assert_eq!(
            std::fs::read(&state).expect("keeps history"),
            b"broken json"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_uses_the_same_reading_position() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let original = dir.path().join("original.pdf");
        let alias = dir.path().join("alias.pdf");
        let state = dir.path().join("pages.json");
        std::fs::write(&original, b"pdf").expect("a file");
        std::os::unix::fs::symlink(&original, &alias).expect("a symlink");
        remember(&state, &original, 9).expect("remembers");
        assert_eq!(page(&state, &alias).expect("alias"), Some(9));
    }
}
