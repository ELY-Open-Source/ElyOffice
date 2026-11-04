//! Reading views stay outside the PDF.

use ely_gpui_component::primitives::Severity;
use ely_office::{Tab, Tabs as _};
use gpui::Context;
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
};

use crate::{
    engine::Pdf,
    layout::{Mode, Zoom},
    side::Pane,
    view::{PdfView, Reported},
    viewer::Viewer,
};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Sidebar {
    pub pane: Pane,
    pub shown: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Viewport {
    mode: Mode,
    zoom: Zoom,
    rotate: i32,
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Reading {
    pub page: usize,
    viewport: Option<Viewport>,
    sidebar: Option<Sidebar>,
}

impl Reading {
    pub(crate) fn at_page(page: usize) -> Self {
        Self {
            page,
            viewport: None,
            sidebar: None,
        }
    }

    fn valid(&self) -> bool {
        self.viewport.is_none_or(|v| {
            let zoom = match v.zoom {
                Zoom::Scale(s) => s.is_finite() && (0.01..=64.0).contains(&s),
                _ => true,
            };
            zoom && (0..360).contains(&v.rotate)
                && v.rotate % 90 == 0
                && v.x.is_finite()
                && v.y.is_finite()
                && v.x.abs() <= 1_000_000.0
                && v.y.abs() <= 1_000_000.0
        })
    }

    fn capture(view: &PdfView) -> Self {
        if let Some(reading) = view.restore_view {
            return reading;
        }
        let laid = view.laid_out();
        let offset = view.scroll.offset();
        let (left, top) = view
            .go_to
            .unwrap_or((-f32::from(offset.x), -f32::from(offset.y)));
        let viewport = laid
            .placed
            .iter()
            .find(|p| p.page == view.current)
            .map(|p| Viewport {
                mode: view.mode,
                zoom: view.zoom,
                rotate: view.rotate,
                x: (left - p.x) / p.w,
                y: (top - p.y) / p.h,
            });
        Self {
            page: view.current,
            viewport,
            sidebar: view.sidebar,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Kept {
    Page(usize),
    View(Reading),
}

type Pages = Vec<(PathBuf, Kept)>;

fn read(file: &Path) -> io::Result<Pages> {
    let pages: Pages = match std::fs::read(file) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other)?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    if pages
        .iter()
        .any(|(_, value)| matches!(value, Kept::View(v) if !v.valid()))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid reading view",
        ));
    }
    Ok(pages)
}

fn page(file: &Path, path: &Path) -> io::Result<Option<Reading>> {
    let path = std::fs::canonicalize(path)?;
    Ok(read(file)?
        .into_iter()
        .find(|(known, _)| *known == path)
        .map(|(_, kept)| match kept {
            Kept::Page(page) => Reading::at_page(page),
            Kept::View(view) => view,
        }))
}

fn remember(file: &Path, path: &Path, reading: Reading) -> io::Result<()> {
    if !reading.valid() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid reading view",
        ));
    }
    let path = std::fs::canonicalize(path)?;
    let mut pages = read(file)?;
    pages.retain(|(known, _)| *known != path);
    pages.insert(0, (path, Kept::View(reading)));
    pages.truncate(1000);
    let bytes = serde_json::to_vec(&pages).map_err(io::Error::other)?;
    ely_files::save_atomic(file, &bytes, 0).map(drop)
}

impl PdfView {
    pub(crate) fn resize_reading(
        &mut self,
        room: gpui::Bounds<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) {
        let held = (self.restore_view.is_none() && self.room.size.width > gpui::px(0.0))
            .then(|| self.layout_anchor.unwrap_or_else(|| Reading::capture(self)));
        self.room = room;
        if let Some(reading) = held {
            self.restore_reading(reading, cx);
        } else {
            cx.notify();
        }
    }

    pub(crate) fn restore_reading(&mut self, reading: Reading, cx: &mut Context<Self>) {
        let page = reading.page.min(self.sizes.len().saturating_sub(1));
        self.show(page, 0.0, cx);
        if let Some(saved) = reading.viewport {
            let laid = self.laid_out();
            if let Some(p) = laid.placed.iter().find(|p| p.page == page) {
                let left = (p.x + saved.x * p.w).clamp(
                    0.0,
                    (laid.size.0 - f32::from(self.room.size.width)).max(0.0),
                );
                let top = (p.y + saved.y * p.h).clamp(
                    0.0,
                    (laid.size.1 - f32::from(self.room.size.height)).max(0.0),
                );
                self.go_to = Some((left, top));
            }
        }
        self.layout_anchor = Some(match reading.viewport {
            Some(_) => reading,
            None => Reading::capture(self),
        });
        cx.emit(Reported);
        cx.notify();
    }
}

impl Viewer {
    fn reading_sidebar(&self, cx: &gpui::App) -> Sidebar {
        Sidebar {
            pane: self.side.read(cx).pane,
            shown: ely_ui::settings::panels::shown(true, cx),
        }
    }

    pub(crate) fn show_reading_sidebar(&mut self, id: u64, cx: &mut Context<Self>) {
        let previous = self.reading_sidebar(cx);
        if let Some(view) = self
            .sidebar_tab
            .and_then(|old| self.tab(old))
            .map(|tab| tab.view.clone())
        {
            view.update(cx, |view, _| view.sidebar = Some(previous));
        }
        self.sidebar_tab = Some(id);
        let Some(view) = self.tab(id).map(|tab| tab.view.clone()) else {
            return;
        };
        let sidebar = view.read(cx).sidebar.unwrap_or(Sidebar {
            pane: Pane::Pages,
            shown: previous.shown,
        });
        view.update(cx, |view, _| view.sidebar = Some(sidebar));
        self.side.update(cx, |side, cx| {
            side.pane = sidebar.pane;
            cx.notify();
        });
        ely_ui::settings::panels::show(true, sidebar.shown, cx);
    }

    pub(crate) fn restore_reading_view(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(tab) = self.tab(id) else {
            return;
        };
        let Some(path) = tab.session.path() else {
            return;
        };
        let file = self.core.work.with_file_name("reading-pages.json");
        match page(&file, path) {
            Ok(Some(reading)) => {
                tab.view.update(cx, |view, cx| {
                    if let Some(v) = reading.viewport {
                        (view.mode, view.zoom, view.rotate) = (v.mode, v.zoom, v.rotate);
                    }
                    view.sidebar = reading.sidebar;
                    view.restore_view = Some(reading);
                    cx.notify();
                });
                log::info!("pdf: restores a reading view");
            }
            Ok(None) => {}
            Err(error) => self.reading_failed(&error, cx),
        }
    }

    pub(crate) fn keep_reading_view(&self, tab: &Tab<(), Pdf>, cx: &mut Context<Self>) {
        let Some(path) = tab.session.path() else {
            return;
        };
        let mut reading = Reading::capture(tab.view.read(cx));
        if self.sidebar_tab == Some(tab.id) {
            reading.sidebar = Some(self.reading_sidebar(cx));
        }
        let file = self.core.work.with_file_name("reading-pages.json");
        match remember(&file, path, reading) {
            Ok(()) => log::info!("pdf: remembers a reading view"),
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
        remember(&state, &a, Reading::at_page(4)).expect("remembers");
        remember(&state, &b, Reading::at_page(8)).expect("remembers");
        remember(&state, &a, Reading::at_page(2)).expect("updates");
        assert_eq!(
            page(&state, &a).expect("reopens"),
            Some(Reading::at_page(2))
        );
        assert_eq!(
            page(&state, &b).expect("reopens"),
            Some(Reading::at_page(8))
        );
        assert_eq!(read(&state).expect("history").len(), 2);
        assert_eq!(std::fs::read(&a).expect("unchanged"), b"a");
        std::fs::write(&state, b"broken json").expect("corrupts");
        assert!(page(&state, &a).is_err());
        assert!(remember(&state, &a, Reading::at_page(0)).is_err());
        assert_eq!(
            std::fs::read(&state).expect("keeps history"),
            b"broken json"
        );
    }

    #[test]
    fn full_views_round_trip_and_old_page_numbers_stay_readable() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let file = dir.path().join("pages.json");
        let path = dir.path().join("a.pdf");
        std::fs::write(&path, b"pdf").expect("a file");
        let path = std::fs::canonicalize(path).expect("canonical path");
        std::fs::write(
            &file,
            serde_json::to_vec(&[(&path, 7)]).expect("old history"),
        )
        .expect("writes");
        assert_eq!(
            page(&file, &path).expect("migrates"),
            Some(Reading::at_page(7))
        );
        let mut reading = Reading {
            page: 4,
            viewport: Some(Viewport {
                mode: Mode::Horizontal,
                zoom: Zoom::Scale(2.0),
                rotate: 90,
                x: 0.2,
                y: 0.4,
            }),
            sidebar: Some(Sidebar {
                pane: Pane::Notes,
                shown: false,
            }),
        };
        remember(&file, &path, reading).expect("keeps full view");
        assert_eq!(page(&file, &path).expect("round trip"), Some(reading));
        reading.viewport.as_mut().expect("viewport").zoom = Zoom::Scale(-1.0);
        let before = std::fs::read(&file).expect("history");
        assert!(remember(&file, &path, reading).is_err());
        assert_eq!(std::fs::read(&file).expect("unchanged"), before);
        std::fs::write(
            &file,
            serde_json::to_vec(&[(&path, Kept::View(reading))]).expect("bad history"),
        )
        .expect("writes");
        assert!(page(&file, &path).is_err());
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
        remember(&state, &original, Reading::at_page(9)).expect("remembers");
        assert_eq!(
            page(&state, &alias).expect("alias"),
            Some(Reading::at_page(9))
        );
    }
}
