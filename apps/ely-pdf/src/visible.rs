//! Visible signatures: drawn, typed or pictured (PDF-12).

use std::{io, path::PathBuf};

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    data_display::DescriptionList,
    forms::{Choice, Input, Select, SignaturePad, Stroke, TextInput},
    overlays::Dialog as Card,
    theme::ControlSize,
};
use ely_office::Closing as _;
use ely_pdf_engine::{NewAnnot, Shape, Writing, Wrote};
use gpui::{AnyElement, App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, SharedString, Styled as _, Window, div};

use crate::content::Tool;
use crate::view::PdfView;
use crate::viewer::Viewer;

/// The width a click places, in points.
const WIDTH: f32 = 160.0;

/// A visible signature ready to place, in its own units.
#[derive(Clone, Debug, PartialEq)]
pub enum Stamp {
    /// Strokes scaled to fit a 1×`aspect` box.
    Ink { strokes: Vec<Vec<(f32, f32)>>, aspect: f32 },
    Typed { text: String, font: SharedString },
    Picture { path: PathBuf, aspect: f32 },
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Draw,
    Type,
    Picture,
}

/// The dialog's state while it is open.
pub struct Making {
    mode: Mode,
    pub(crate) strokes: Vec<Stroke>,
    name: Entity<TextInput>,
    font: SharedString,
    picture: Option<(PathBuf, f32)>,
}

/// Points of each stroke.
type Strokes = Vec<Vec<(f32, f32)>>;

/// Strokes moved to the corner, the longer side 1.
pub fn fitted(strokes: &[Vec<(f32, f32)>]) -> Option<(Strokes, f32)> {
    if strokes.iter().all(|stroke| stroke.len() < 2) {
        return None;
    }
    let points = || strokes.iter().flatten();
    let (x0, y0) = points().fold((f32::MAX, f32::MAX), |(x, y), p| (x.min(p.0), y.min(p.1)));
    let (x1, y1) = points().fold((f32::MIN, f32::MIN), |(x, y), p| (x.max(p.0), y.max(p.1)));
    // The longer side spans 1, so tall strokes fit too.
    let width = (x1 - x0).max(y1 - y0).max(1.0);
    let fitted = strokes.iter().map(|stroke| stroke.iter().map(|p| ((p.0 - x0) / width, (p.1 - y0) / width)).collect()).collect();
    Some((fitted, (y1 - y0) / width))
}

/// A stamp's box: fitted to a drag, or click sized.
pub fn placed(aspect: f32, rect: [f32; 4], click: bool) -> [f32; 4] {
    let (w, h) = (rect[2] - rect[0], rect[3] - rect[1]);
    let width = if click { WIDTH } else { w.min(h / aspect.max(0.01)) };
    [rect[0], rect[1], rect[0] + width, rect[1] + width * aspect]
}

impl PdfView {
    /// Places the ready stamp at `rect` on `page`.
    pub fn place_stamp(&mut self, page: usize, rect: [f32; 4], click: bool, cx: &mut Context<Self>) {
        let Some(stamp) = self.stamp.take() else {
            return;
        };
        self.set_tool(Tool::Read, cx);
        match stamp {
            Stamp::Ink { strokes, aspect } => {
                let at = placed(aspect, rect, click);
                let scale = at[2] - at[0];
                let strokes = strokes.iter().map(|stroke| stroke.iter().map(|p| (at[0] + p.0 * scale, at[1] + p.1 * scale)).collect()).collect();
                let annot = NewAnnot { page, shape: Shape::Ink(strokes), rgb: Some(0), contents: String::new(), author: String::new() };
                self.annotate("pdf.doing.sign_drawn", vec![annot], cx);
            }
            Stamp::Typed { text, font } => {
                let at = placed(0.35, rect, click);
                let writing = Writing {
                    block: None,
                    expected: String::new(),
                    text,
                    font: font.to_string(),
                    size: (at[3] - at[1]) * 0.8,
                    rgb: 0,
                    bx: [at[0], at[1], self.sizes[page].0 - at[0]],
                };
                let write = move |worker: &mut ely_pdf_engine::Worker, doc| match worker.put_text(doc, page, &writing, 0.0)? {
                    Wrote::Done(mark) => Ok(mark),
                    Wrote::Missing(c) => Err(crate::writes::no_glyph(c)),
                    Wrote::Unwritable => Err(crate::writes::unwritable()),
                    Wrote::Preview(..) => Err(io::Error::other("a preview where a write was asked")),
                };
                self.edit_at("pdf.doing.sign_typed", write, cx);
            }
            Stamp::Picture { path, aspect } => {
                let at = placed(aspect, rect, click);
                self.edit_at("pdf.doing.sign_picture", move |worker, doc| worker.add_image(doc, page, &path, at), cx);
            }
        }
    }
}

impl Viewer {
    pub fn ask_visible(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_view().is_none() {
            return self.no_document(cx);
        }
        let name = cx.new(|cx| TextInput::new(window, cx));
        self.making = Some(Making { mode: Mode::Draw, strokes: Vec::new(), name, font: "script".into(), picture: None });
        cx.notify();
    }

    fn choose_picture(&mut self, cx: &mut Context<Self>) {
        self.pick(false, cx, |viewer, path, cx| {
            match image::image_dimensions(&path) {
                Ok((w, h)) if w > 0 => {
                    if let Some(making) = viewer.making.as_mut() {
                        making.picture = Some((path, h as f32 / w as f32));
                    }
                }
                Ok(_) => viewer.panel_failed(&ely_ui::i18n::text(cx, "error.no_pixels", &[]), cx),
                Err(error) => viewer.panel_failed(&error.to_string(), cx),
            }
            cx.notify();
        });
    }

    /// The stamp the dialog describes, if it describes one.
    fn stamp(&self, cx: &App) -> Option<Stamp> {
        let making = self.making.as_ref()?;
        match making.mode {
            Mode::Draw => {
                let strokes: Vec<Vec<(f32, f32)>> =
                    making.strokes.iter().map(|stroke| stroke.iter().map(|p| (f32::from(p.x), f32::from(p.y))).collect()).collect();
                let (strokes, aspect) = fitted(&strokes)?;
                Some(Stamp::Ink { strokes, aspect })
            }
            Mode::Type => {
                let text = making.name.read(cx).text().trim().to_string();
                (!text.is_empty()).then(|| Stamp::Typed { text, font: making.font.clone() })
            }
            Mode::Picture => making.picture.clone().map(|(path, aspect)| Stamp::Picture { path, aspect }),
        }
    }

    /// Hands the stamp to the view's next press.
    pub(crate) fn ready_stamp(&mut self, cx: &mut Context<Self>) {
        let (Some(stamp), Some(view)) = (self.stamp(cx), self.active_view()) else {
            return;
        };
        self.making = None;
        view.update(cx, |view, cx| {
            view.set_tool(Tool::PlaceSignature, cx);
            view.stamp = Some(stamp);
        });
        cx.notify();
    }

    fn set_making(&mut self, cx: &mut Context<Self>, change: impl FnOnce(&mut Making)) {
        if let Some(making) = self.making.as_mut() {
            change(making);
        }
        cx.notify();
    }

    /// The visible signature dialog, while open.
    pub fn visible_element(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let making = self.making.as_ref()?;
        let this = cx.entity();
        let label = |key: &str| Self::text(cx, key, &[]);
        let switch = this.clone();
        let mode = match making.mode {
            Mode::Draw => "draw",
            Mode::Type => "type",
            Mode::Picture => "picture",
        };
        let modes = SegmentedControl::new("pdf-visible-mode", mode)
            .size(ControlSize::Sm)
            .segment("draw", label("pdf.visible.draw"), None)
            .segment("type", label("pdf.visible.type"), None)
            .segment("picture", label("pdf.visible.picture"), None)
            .on_change(move |value, _, cx| {
                let mode = match value.as_ref() {
                    "type" => Mode::Type,
                    "picture" => Mode::Picture,
                    _ => Mode::Draw,
                };
                switch.update(cx, |viewer, cx| viewer.set_making(cx, |making| making.mode = mode))
            });
        let body = match making.mode {
            Mode::Draw => {
                let drawn = this.clone();
                SignaturePad::new("pdf-visible-pad", making.strokes.clone())
                    .on_change(move |strokes, _, cx| {
                        let strokes = strokes.to_vec();
                        drawn.update(cx, |viewer, cx| viewer.set_making(cx, |making| making.strokes = strokes))
                    })
                    .into_any_element()
            }
            Mode::Type => {
                let fonts = std::iter::once(("script", "pdf.font.script"))
                    .chain(crate::editing::FONTS.iter().copied().filter(|(value, _)| *value != "original"))
                    .map(|(value, key)| Choice::new(value, label(key)))
                    .collect::<Vec<_>>();
                let picked = this.clone();
                let font = Select::new("pdf-visible-font", fonts)
                    .selected(making.font.clone())
                    .label(label("pdf.text.font"))
                    .on_change(move |value, _, cx| {
                        let value = value.clone();
                        picked.update(cx, |viewer, cx| viewer.set_making(cx, |making| making.font = value))
                    });
                DescriptionList::new()
                    .item(label("pdf.visible.name"), div().w_full().child(Input::new(&making.name)))
                    .item(label("pdf.text.font"), font)
                    .into_any_element()
            }
            Mode::Picture => {
                let chosen = making.picture.as_ref().map_or_else(|| label("pdf.visible.no_picture"), |(path, _)| ely_office::files::file_name(path));
                let pick = this.clone();
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(div().flex_1().child(chosen))
                    .child(Button::new("pdf-visible-pick", label("pdf.sign.choose")).on_click(move |_, _, cx| pick.update(cx, |viewer, cx| viewer.choose_picture(cx))))
                    .into_any_element()
            }
        };
        let closed = this.clone();
        let dismiss = move |_: &mut Window, cx: &mut App| {
            closed.update(cx, |viewer, cx| {
                viewer.making = None;
                cx.notify();
            })
        };
        let (ready, go) = (self.stamp(cx).is_some(), label("pdf.visible.place"));
        let card = Card::new("pdf-visible", label("pdf.visible.title"), dismiss)
            .detail(label("pdf.visible.detail"))
            .child(div().flex().flex_col().gap_3().child(modes).child(body))
            .action(ely_office::cancel(cx))
            .action(move |_| {
                Button::new("pdf-visible-go", go)
                    .variant(ButtonVariant::Primary)
                    .disabled(!ready)
                    .on_click(move |_, _, cx| this.update(cx, |viewer, cx| viewer.ready_stamp(cx)))
            });
        Some(card.into_any_element())
    }
}
