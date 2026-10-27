//! Page organizing: insert, delete, move, turn, undo (PDF-02).

use ely_gpui_component::secondary;
use std::{collections::BTreeSet, path::PathBuf};

use ely_gpui_component::primitives::Severity;
use ely_office::{Closing as _, Tabs as _};
use gpui::{App, Context, Div, InteractiveElement as _, KeyBinding, Menu, MenuItem, actions};

use crate::view::PdfView;
use crate::viewer::Viewer;

actions!(
    pdf,
    [
        Undo,
        Redo,
        InsertBlankPage,
        InsertPagesFromFile,
        DeletePages,
        RotatePagesLeft,
        RotatePagesRight,
        MovePagesUp,
        MovePagesDown,
        MovePagesFirst,
        MovePagesLast,
        CropPages,
        ExtractPages,
        SplitDocument
    ]
);

pub fn bind(cx: &mut App) {
    let at = Some(ely_ui::commands::CONTEXT);
    cx.bind_keys([
        KeyBinding::new(secondary!("z"), Undo, at),
        KeyBinding::new(secondary!("shift-z"), Redo, at),
        KeyBinding::new("backspace", DeletePages, Some(crate::side::PAGES)),
        KeyBinding::new("delete", DeletePages, Some(crate::side::PAGES)),
    ]);
}

/// The Pages menu.
pub fn menu(cx: &App) -> Menu {
    let label = |key: &str| Viewer::text(cx, key, &[]);
    Menu::new(label("menu.pages")).items([
        MenuItem::action(label("pdf.pages.blank"), InsertBlankPage),
        MenuItem::action(label("pdf.pages.file"), InsertPagesFromFile),
        MenuItem::action(label("pdf.pages.delete"), DeletePages),
        MenuItem::separator(),
        MenuItem::action(label("pdf.pages.rotate_left"), RotatePagesLeft),
        MenuItem::action(label("pdf.pages.rotate_right"), RotatePagesRight),
        MenuItem::action(label("pdf.pages.crop"), CropPages),
        MenuItem::separator(),
        MenuItem::action(label("pdf.pages.up"), MovePagesUp),
        MenuItem::action(label("pdf.pages.down"), MovePagesDown),
        MenuItem::action(label("pdf.pages.first"), MovePagesFirst),
        MenuItem::action(label("pdf.pages.last"), MovePagesLast),
        MenuItem::separator(),
        MenuItem::action(label("pdf.pages.extract"), ExtractPages),
        MenuItem::action(label("pdf.pages.split"), SplitDocument),
    ])
}

/// The Edit menu's undo and redo.
pub fn history_items(cx: &App) -> [MenuItem; 2] {
    let label = |key: &str| Viewer::text(cx, key, &[]);
    [MenuItem::action(label("pdf.undo"), Undo), MenuItem::action(label("pdf.redo"), Redo)]
}

impl PdfView {
    /// The pages an organizing step acts on: picked, else current.
    pub fn chosen_pages(&self) -> Vec<usize> {
        match self.picked.is_empty() {
            true => vec![self.current],
            false => self.picked.iter().copied().collect(),
        }
    }

    /// Moves the chosen pages before page `to`; they stay picked.
    pub fn move_to(&mut self, to: usize, cx: &mut Context<Self>) {
        let pages = self.chosen_pages();
        let before = pages.iter().filter(|page| **page < to).count();
        let landed: BTreeSet<usize> = (to - before..to - before + pages.len()).collect();
        if landed.iter().eq(pages.iter()) {
            return log::info!("pdf: pages already at {to}; nothing moves");
        }
        self.queue_edit("moving pages", Some(landed), true, move |worker, doc| worker.move_pages(doc, &pages, to), cx);
    }
}

impl Viewer {
    /// Runs `step` on the active view; with none, says so.
    fn on_pages(&mut self, cx: &mut Context<Self>, step: impl FnOnce(&mut PdfView, &mut Context<PdfView>)) {
        match self.active_view() {
            Some(view) => view.update(cx, step),
            None => self.no_document(cx),
        }
    }

    pub(crate) fn delete(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.active_view() else {
            return self.no_document(cx);
        };
        let (pages, count) = (view.read(cx).chosen_pages(), view.read(cx).sizes.len());
        if pages.len() == count {
            let text = Self::text(cx, "pdf.pages.keep_one", &[]);
            return self.toast(text, Severity::Warning, cx);
        }
        view.update(cx, |view, cx| {
            view.picked.clear();
            view.edit_pages("deleting pages", move |worker, doc| worker.delete_pages(doc, &pages), cx);
        });
    }

    fn turn_pages(&mut self, by: i32, cx: &mut Context<Self>) {
        self.on_pages(cx, |view, cx| {
            let pages = view.chosen_pages();
            view.edit_pages("turning pages", move |worker, doc| worker.rotate_pages(doc, &pages, by), cx);
        });
    }

    /// Moves the chosen pages by `by`, or to an end.
    fn shift(&mut self, by: isize, cx: &mut Context<Self>) {
        self.on_pages(cx, |view, cx| {
            let (pages, count) = (view.chosen_pages(), view.sizes.len() as isize);
            let to = match by {
                isize::MIN => 0,
                isize::MAX => count,
                -1 => pages[0] as isize - 1,
                _ => *pages.last().expect("one page at least") as isize + 2,
            };
            if (0..=count).contains(&to) {
                view.move_to(to as usize, cx);
            }
        });
    }

    fn blank(&mut self, cx: &mut Context<Self>) {
        self.on_pages(cx, |view, cx| {
            let at = view.chosen_pages().last().copied().expect("one page at least");
            let size = view.sizes[at];
            view.edit_pages("inserting a page", move |worker, doc| worker.insert_blank(doc, at + 1, size), cx);
        });
    }

    /// Asks for a PDF; its pages follow the chosen.
    fn insert_file(&mut self, cx: &mut Context<Self>) {
        if self.active_view().is_none() {
            return self.no_document(cx);
        }
        self.pick(false, cx, |viewer, path: PathBuf, cx| {
            viewer.on_pages(cx, |view, cx| {
                let at = view.chosen_pages().last().copied().expect("one page at least") + 1;
                view.edit_pages(
                    "inserting pages",
                    move |worker, doc| worker.insert_file(doc, at, &path).map(|(_, mark)| mark),
                    cx,
                );
            })
        });
    }
}

/// Wires the organizing actions onto the root.
pub fn on(root: Div, cx: &mut Context<Viewer>) -> Div {
    root.on_action(cx.listener(|v, _: &Undo, _, cx| v.on_pages(cx, |view, cx| view.undo(false, cx))))
        .on_action(cx.listener(|v, _: &Redo, _, cx| v.on_pages(cx, |view, cx| view.undo(true, cx))))
        .on_action(cx.listener(|v, _: &DeletePages, _, cx| v.delete(cx)))
        .on_action(cx.listener(|v, _: &RotatePagesLeft, _, cx| v.turn_pages(-90, cx)))
        .on_action(cx.listener(|v, _: &RotatePagesRight, _, cx| v.turn_pages(90, cx)))
        .on_action(cx.listener(|v, _: &MovePagesUp, _, cx| v.shift(-1, cx)))
        .on_action(cx.listener(|v, _: &MovePagesDown, _, cx| v.shift(1, cx)))
        .on_action(cx.listener(|v, _: &MovePagesFirst, _, cx| v.shift(isize::MIN, cx)))
        .on_action(cx.listener(|v, _: &MovePagesLast, _, cx| v.shift(isize::MAX, cx)))
        .on_action(cx.listener(|v, _: &InsertBlankPage, _, cx| v.blank(cx)))
        .on_action(cx.listener(|v, _: &InsertPagesFromFile, _, cx| v.insert_file(cx)))
        .on_action(cx.listener(|v, _: &CropPages, window, cx| v.ask_crop(window, cx)))
        .on_action(cx.listener(|v, _: &ExtractPages, _, cx| v.extract(cx)))
        .on_action(cx.listener(|v, _: &SplitDocument, window, cx| v.ask_split(window, cx)))
}
