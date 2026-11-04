//! Writing text, previews, objects and new images (PDF-03, PDF-04).

use std::{io, path::PathBuf, sync::Arc};

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    overlays::Dialog as Card,
    primitives::Severity,
};
use ely_office::{Closing as _, Tabs as _};
use ely_pdf_engine::{Writing, Wrote};
use ely_ipc::Worded;
use gpui::{
    AnyElement, App, Context, IntoElement, KeyBinding, MenuItem, ParentElement as _, Styled as _, Window, actions,
    div, img, px,
};

use crate::content::{Aim, Tool};
use crate::editing::Preview;
use crate::viewer::Viewer;

actions!(pdf, [EditContent, AddText, AddImage, StopEditing, DeleteObject]);

/// The key context while an editing tool is on.
pub const EDITING: &str = "PdfEditing";

pub fn bind(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", StopEditing, Some(EDITING)),
        KeyBinding::new("delete", DeleteObject, Some(EDITING)),
        KeyBinding::new("backspace", DeleteObject, Some(EDITING)),
    ]);
}

/// The editing items of the Edit menu.
pub fn items(cx: &App) -> [MenuItem; 3] {
    let label = |key: &str| Viewer::text(cx, key, &[]);
    [
        MenuItem::action(label("pdf.tool.edit"), EditContent),
        MenuItem::action(label("pdf.tool.text"), AddText),
        MenuItem::action(label("pdf.tool.image"), AddImage),
    ]
}

/// Pixels a point in a preview.
const PREVIEW: f32 = 2.0;

/// The chosen font cannot draw `c`.
pub(crate) fn no_glyph(c: char) -> io::Error {
    Worded::error(io::ErrorKind::InvalidInput, "pdf.text.missing", &[("char", &c.to_string())])
}

/// The document's own font cannot write new text.
pub(crate) fn unwritable() -> io::Error {
    Worded::error(io::ErrorKind::InvalidInput, "pdf.text.unwritable", &[])
}

impl Viewer {
    /// Turns `tool` on, or off when it is on.
    pub(crate) fn toggle_tool(&mut self, tool: Tool, cx: &mut Context<Self>) {
        let Some(view) = self.active_view() else {
            return self.no_document(cx);
        };
        view.update(cx, |view, cx| {
            let now = if view.tool == tool { Tool::Read } else { tool };
            view.set_tool(now, cx)
        });
    }

    /// Writes the panel's text, or empties the block when `erase`.
    pub fn write_text(&mut self, erase: bool, cx: &mut Context<Self>) {
        let (Some(tab), Some(view)) = (self.core.active(), self.active_view()) else {
            return;
        };
        let Some(aim) = view.read(cx).aim else {
            return;
        };
        let Some((page, mut writing)) = self.edit_panel.read(cx).writing(view.read(cx), aim, cx) else {
            return;
        };
        if erase {
            writing.text.clear();
        }
        // A replacement that changes the layout is shown first.
        let (replacing, version) = (matches!(aim, Aim::Text { .. }), view.read(cx).version);
        let (asked, viewer) = (writing.clone(), cx.weak_entity());
        view.update(cx, |view, cx| {
            let doc = view.doc;
            // A stale answer still reaches `previewed`, which says so.
            view.off(
                move |worker| worker.put_text(doc, page, &asked, PREVIEW),
                move |_, answer, cx| {
                    // The viewer edits this view; wait until it is free.
                    cx.defer(move |cx| {
                        if let Some(viewer) = viewer.upgrade() {
                            viewer.update(cx, |viewer, cx| viewer.previewed((tab, version, page), writing, answer, replacing, cx));
                        }
                    })
                },
                cx,
            );
        });
    }

    /// Shows a layout change, a missing glyph, or writes.
    fn previewed(
        &mut self,
        (tab, version, page): (u64, u64, usize),
        writing: Writing,
        answer: io::Result<Wrote>,
        replacing: bool,
        cx: &mut Context<Self>,
    ) {
        let same = self.tab(tab).is_some_and(|open| open.view.read(cx).version == version);
        if !same {
            let text = Self::text(cx, "pdf.object.changed", &[]);
            return self.toast(text, Severity::Warning, cx);
        }
        match answer {
            Ok(Wrote::Unwritable) => {
                let note = Self::text(cx, "pdf.text.unwritable", &[]);
                self.edit_panel.update(cx, |panel, cx| {
                    panel.note = Some(note);
                    cx.notify();
                });
            }
            Ok(Wrote::Missing(c)) => {
                let note = Self::text(cx, "pdf.text.missing", &[("char", &c.to_string())]);
                self.edit_panel.update(cx, |panel, cx| {
                    panel.note = Some(note);
                    cx.notify();
                });
            }
            Ok(Wrote::Preview(pixels, size, grew)) if replacing && (grew || writing.font != "original") => {
                let Some(image) = crate::pages::picture(size, pixels) else {
                    return log::error!("pdf: a preview came back the wrong size");
                };
                self.end_preview();
                self.preview = Some(Preview { tab, version, page, writing, image });
                cx.notify();
            }
            Ok(_) => self.commit_text((tab, version, page), writing, cx),
            Err(error) => self.failed_read("pdf.doing.preview", &error, cx),
        }
    }

    /// Writes for real if tab `tab` is still at `version`.
    pub fn commit_text(&mut self, (tab, version, page): (u64, u64, usize), writing: Writing, cx: &mut Context<Self>) {
        let Some(view) = self.tab(tab).map(|tab| tab.view.clone()) else {
            let text = Self::text(cx, "pdf.object.changed", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        self.on_same(&view, version, cx, |view, cx| {
            view.edit(
                "pdf.doing.write",
                move |worker, doc| match worker.put_text(doc, page, &writing, 0.0)? {
                    Wrote::Done(mark) => Ok(mark),
                    Wrote::Missing(c) => Err(no_glyph(c)),
                    Wrote::Unwritable => Err(unwritable()),
                    Wrote::Preview(..) => Err(io::Error::other("a write came back as a preview")),
                },
                cx,
            )
        });
    }

    fn failed_read(&self, what: &str, error: &io::Error, cx: &mut App) {
        log::error!("pdf: {what} failed ({:?})", error.kind());
        let text = Self::text(cx, "pdf.failed", &[("what", &Self::text(cx, what, &[])), ("error", &ely_ui::i18n::error(cx, error))]);
        self.toast_unlogged(text, Severity::Danger, cx);
    }

    /// The aimed object: its page and index.
    fn aimed_object(&self, cx: &App) -> Option<(usize, usize)> {
        match self.active_view()?.read(cx).aim? {
            Aim::Object { page, index } => Some((page, index)),
            _ => None,
        }
    }

    pub fn delete_object(&mut self, cx: &mut Context<Self>) {
        let (Some((page, index)), Some(view)) = (self.aimed_object(cx), self.active_view()) else {
            return;
        };
        let version = view.read(cx).version;
        self.on_same(&view, version, cx, |view, cx| {
            view.edit("pdf.doing.delete_object", move |worker, doc| worker.delete_object(doc, page, index), cx)
        });
    }

    /// Asks for a picture to take the aimed image's place.
    pub fn replace_image(&mut self, cx: &mut Context<Self>) {
        let (Some((page, index)), Some(view)) = (self.aimed_object(cx), self.active_view()) else {
            return;
        };
        let version = view.read(cx).version;
        self.pick(false, cx, move |viewer, path: PathBuf, cx| {
            viewer.on_same(&view, version, cx, |view, cx| {
                view.edit("pdf.doing.replace_image", move |worker, doc| worker.replace_image(doc, page, index, &path), cx)
            });
        });
    }

    /// Whether `step` ran: no edit came since `version`.
    pub(crate) fn on_same(
        &self,
        view: &gpui::Entity<crate::view::PdfView>,
        version: u64,
        cx: &mut Context<Self>,
        step: impl FnOnce(&mut crate::view::PdfView, &mut Context<crate::view::PdfView>),
    ) -> bool {
        // Landed or queued, an edit since may move the target.
        let open = self.core.tabs.iter().any(|tab| tab.view == *view);
        if !open || view.read(cx).version != version || !view.read(cx).settled() {
            let text = Self::text(cx, "pdf.object.changed", &[]);
            self.toast(text, Severity::Warning, cx);
            return false;
        }
        view.update(cx, step);
        true
    }

    /// Asks for a picture; it lands mid-page.
    fn add_image(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.active_view() else {
            return self.no_document(cx);
        };
        let (version, page) = (view.read(cx).version, view.read(cx).current);
        self.pick(false, cx, move |viewer, path: PathBuf, cx| {
            let (w, h) = match image::image_dimensions(&path) {
                Ok(size) => size,
                Err(error) => return viewer.failed_read("pdf.doing.picture", &io::Error::other(error), cx),
            };
            viewer.on_same(&view, version, cx, |view, cx| {
                let (pw, ph) = view.sizes[page];
                // Pixels as points, shrunk to half the page.
                let fit = (pw * 0.5 / w as f32).min(ph * 0.5 / h as f32).min(0.75);
                let (iw, ih) = (w as f32 * fit, h as f32 * fit);
                let rect = [(pw - iw) / 2.0, (ph - ih) / 2.0, (pw + iw) / 2.0, (ph + ih) / 2.0];
                view.edit("pdf.doing.add_image", move |worker, doc| worker.add_image(doc, page, &path, rect), cx)
            });
        });
    }

    /// The preview of a layout change, while open.
    pub fn preview_element(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let preview = self.preview.as_ref()?;
        let this = cx.entity();
        let dismiss = {
            let this = this.clone();
            move |_: &mut Window, cx: &mut App| {
                this.update(cx, |viewer, cx| {
                    viewer.end_preview();
                    cx.notify();
                })
            }
        };
        let go = Self::text(cx, "pdf.text.apply", &[]);
        let picture = img(Arc::clone(&preview.image)).max_w(px(560.0)).max_h(px(420.0));
        let card = Card::new("pdf-preview", Self::text(cx, "pdf.preview.title", &[]), dismiss)
            .detail(Self::text(cx, "pdf.preview.detail", &[]))
            .child(div().flex().justify_center().child(picture))
            .action(ely_office::cancel(cx))
            .action(move |_| {
                Button::new("pdf-preview-go", go).variant(ButtonVariant::Primary).on_click(move |_, _, cx| {
                    this.update(cx, |viewer, cx| {
                        if let Some(Preview { tab, version, page, writing, image }) = viewer.preview.take() {
                            viewer.retired.push(image);
                            viewer.commit_text((tab, version, page), writing, cx);
                        }
                        cx.notify();
                    })
                })
            });
        Some(card.into_any_element())
    }
}

/// Wires the editing actions onto the root.
pub fn on(root: gpui::Div, cx: &mut Context<Viewer>) -> gpui::Div {
    use gpui::InteractiveElement as _;
    root.on_action(cx.listener(|v, _: &EditContent, _, cx| v.toggle_tool(Tool::Edit, cx)))
        .on_action(cx.listener(|v, _: &AddText, _, cx| v.toggle_tool(Tool::AddText, cx)))
        .on_action(cx.listener(|v, _: &AddImage, _, cx| v.add_image(cx)))
        .on_action(cx.listener(|v, _: &StopEditing, _, cx| v.on_view(|view, cx| view.set_tool(Tool::Read, cx), cx)))
        .on_action(cx.listener(|v, _: &DeleteObject, _, cx| v.delete_object(cx)))
}

impl Viewer {
    /// Drops the text preview; its picture is freed next render.
    pub fn end_preview(&mut self) {
        if let Some(preview) = self.preview.take() {
            self.retired.push(preview.image);
        }
    }
}
