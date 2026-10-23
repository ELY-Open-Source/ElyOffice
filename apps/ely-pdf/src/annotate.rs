//! Annotation and form tools on the page (PDF-05, PDF-06).

use std::collections::HashMap;

use ely_gpui_component::theme::ActiveTheme as _;
use ely_pdf_engine::{FieldKind, Marks, NewAnnot, Shape};
use gpui::{AnyElement, App, Context, IntoElement, Pixels, Point, Styled as _, canvas, div, point, px};

use crate::content::{Aim, Tool};
use crate::layout::Placed;
use crate::pages::{rect_on, to_view};
use crate::view::{PdfView, Unread};

/// How far a click may move, in window pixels.
pub(crate) const CLICK: f32 = 3.0;
/// Pages read for their annotations at once.
const BATCH: usize = 20;

/// What the note panel works on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pick {
    Annot { page: usize, id: u64 },
    Field { page: usize, id: u64 },
    NewNote { page: usize, at: (f32, f32) },
    NewStamp { page: usize, at: (f32, f32) },
    NewField { page: usize, rect: [f32; 4], kind: FieldKind },
    Link { page: usize, index: usize },
    NewLink { page: usize, rect: [f32; 4] },
}

/// A shape being drawn, in page points.
#[derive(Clone, Debug, PartialEq)]
pub struct Sketch {
    pub page: usize,
    pub points: Vec<(f32, f32)>,
}

impl Sketch {
    /// The box from the first point to the last.
    pub fn rect(&self) -> [f32; 4] {
        let (a, b) = (self.points[0], self.points[self.points.len() - 1]);
        [a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1)]
    }
}

/// Annotations and fields by page.
pub type Notes = HashMap<usize, Marks>;

fn inside(rect: &[f32; 4], (x, y): (f32, f32)) -> bool {
    x >= rect[0] && x <= rect[2] && y >= rect[1] && y <= rect[3]
}

/// A box around `at`, kept on the page.
pub fn around((x, y): (f32, f32), (w, h): (f32, f32), size: (f32, f32)) -> [f32; 4] {
    let left = (x - w / 2.0).clamp(0.0, (size.0 - w).max(0.0));
    let top = (y - h / 2.0).clamp(0.0, (size.1 - h).max(0.0));
    [left, top, left + w, top + h]
}

/// Picked text as one box per line, by page.
pub fn line_boxes(chars: &[ely_pdf_engine::Char]) -> Vec<[f32; 4]> {
    let mut boxes: Vec<(i32, [f32; 4])> = Vec::new();
    for c in chars {
        match boxes.last_mut() {
            Some((line, b)) if *line == c.line => {
                *b = [b[0].min(c.rect[0]), b[1].min(c.rect[1]), b[2].max(c.rect[2]), b[3].max(c.rect[3])];
            }
            _ => boxes.push((c.line, c.rect)),
        }
    }
    boxes.into_iter().map(|(_, b)| b).collect()
}

impl PdfView {
    /// Reads the annotations of up to a batch of `pages`.
    pub fn ask_notes(&mut self, pages: impl IntoIterator<Item = usize>, cx: &mut Context<Self>) {
        if self.reading_notes {
            return;
        }
        let wanted: Vec<usize> = (pages.into_iter())
            .filter(|page| !self.notes.contains_key(page) && !self.unread.contains(&Unread::Notes(*page)))
            .take(BATCH)
            .collect();
        if wanted.is_empty() {
            return;
        }
        let asked = wanted.clone();
        self.reading_notes = true;
        let (doc, version) = (self.doc, self.version);
        self.off(
            // Page by page: one bad page spares the rest.
            move |worker| {
                let read = (wanted.into_iter())
                    .map(|page| (page, worker.marks(doc, page).and_then(|marks| Ok((marks, worker.links(doc, page)?)))))
                    .collect::<Vec<_>>();
                Ok(read)
            },
            move |view, read: std::io::Result<Vec<_>>, cx| {
                view.reading_notes = false;
                if view.version != version {
                    return cx.notify();
                }
                // No worker at all: every page asked failed.
                let read = read.unwrap_or_else(|error| {
                    let why = error.to_string();
                    asked.into_iter().map(|page| (page, Err(std::io::Error::other(why.clone())))).collect()
                });
                let mut first = None;
                for (page, page_read) in read {
                    match page_read {
                        Ok((marks, links)) => {
                            view.notes.insert(page, marks);
                            view.links.insert(page, links);
                        }
                        Err(error) => {
                            view.unread.insert(Unread::Notes(page));
                            first.get_or_insert(error);
                        }
                    }
                }
                if let Some(error) = first {
                    view.fail("reading annotations", &error, cx);
                }
                cx.notify();
            },
            cx,
        );
    }

    /// Reads every field name in the form, once an edit.
    pub fn ask_names(&mut self, cx: &mut Context<Self>) {
        if self.form_names.is_some() || self.reading_names || self.unread.contains(&Unread::Names) {
            return;
        }
        self.reading_names = true;
        let (doc, version) = (self.doc, self.version);
        self.off(
            move |worker| worker.field_names(doc),
            move |view, read, cx| {
                view.reading_names = false;
                if view.version != version {
                    return cx.notify();
                }
                match read {
                    Ok(names) => view.form_names = Some(names),
                    Err(error) => {
                        view.unread.insert(Unread::Names);
                        view.fail("reading field names", &error, cx)
                    }
                }
                cx.notify();
            },
            cx,
        );
    }

    /// Ends a text pick: marks it, or clicks.
    pub fn finish_select(&mut self, at: Point<Pixels>, cx: &mut Context<Self>) {
        self.select_to(at, cx);
        let Some(selection) = self.selection else {
            return;
        };
        // A hand that shakes a little still clicks.
        let moved = self.pressed.take().map(|from| f32::from((at.x - from.x).abs() + (at.y - from.y).abs()));
        let still = moved.is_some_and(|moved| moved < CLICK);
        match self.tool {
            Tool::Read if still => {
                self.selection = None;
                self.click(selection.anchor, cx)
            }
            Tool::Highlight | Tool::Underline | Tool::StrikeOut | Tool::RedactText if !still => {
                self.marking = Some(selection);
                self.mark_pending(cx)
            }
            _ => {}
        }
    }

    /// A click fills a field or picks an annotation.
    fn click(&mut self, (page, at): (usize, (f32, f32)), cx: &mut Context<Self>) {
        let Some(marks) = self.notes.get(&page) else {
            return;
        };
        let field = marks.fields.iter().rev().find(|f| inside(&f.rect, at) && !f.readonly).cloned();
        let annot = marks.annots.iter().rev().find(|a| inside(&a.rect, at)).map(|a| a.id);
        self.aim = match (field, annot) {
            (Some(field), _) => match field.kind {
                FieldKind::Checkbox | FieldKind::Radio => {
                    let value = if field.checked && field.kind == FieldKind::Checkbox { "Off".to_string() } else { field.on };
                    if !(field.checked && field.kind == FieldKind::Radio) {
                        self.settled_edit("filling a field", cx, move |worker, doc| worker.fill(doc, page, field.id, &value));
                    }
                    None
                }
                FieldKind::Text | FieldKind::Choice | FieldKind::List => Some(Aim::Note(Pick::Field { page, id: field.id })),
                FieldKind::Signature | FieldKind::Button => None,
            },
            (None, Some(id)) => Some(Aim::Note(Pick::Annot { page, id })),
            (None, None) => {
                let link = self.links.get(&page).and_then(|links| links.iter().rev().find(|l| inside(&l.rect, at))).cloned();
                if let Some(link) = link {
                    self.follow(&link, cx);
                }
                None
            }
        };
        self.open_note_panel(cx);
        cx.notify();
    }

    /// Queues `op` unless an edit is pending.
    pub fn settled_edit(
        &mut self,
        what: &'static str,
        cx: &mut Context<Self>,
        op: impl FnOnce(&mut ely_pdf_engine::Worker, ely_pdf_engine::Doc) -> std::io::Result<String> + Send + 'static,
    ) {
        // A queued edit may move what the press meant.
        if !self.settled() {
            let text = crate::viewer::Viewer::text(cx, "pdf.object.changed", &[]);
            cx.emit(crate::view::Failed(text.to_string()));
            return cx.notify();
        }
        self.edit(what, op, cx);
    }

    /// Marks the pending pick once its pages' text is read.
    pub fn mark_pending(&mut self, cx: &mut Context<Self>) {
        let Some(selection) = self.marking else {
            return;
        };
        let (first, last) = (selection.anchor.0.min(selection.head.0), selection.anchor.0.max(selection.head.0));
        let missing: Vec<usize> = (first..=last).filter(|page| !self.text.contains_key(page)).collect();
        if !missing.is_empty() {
            for page in missing {
                self.page_text(page, cx);
            }
            return;
        }
        self.marking = None;
        self.mark_selection(selection, cx);
    }

    /// The picked text becomes a highlight, underline or strike.
    fn mark_selection(&mut self, selection: crate::select::Selection, cx: &mut Context<Self>) {
        let Some(span) = selection.span(|page| self.text.get(&page).map(|chars| chars.as_slice())) else {
            return;
        };
        let mut annots = Vec::new();
        for page in span.from.0..=span.to.0 {
            let Some(chars) = self.text.get(&page) else {
                return;
            };
            let Some(range) = span.on(page, chars.len()).filter(|range| !range.is_empty()) else {
                continue;
            };
            let boxes = line_boxes(&chars[range]);
            if self.tool == Tool::RedactText {
                self.mark_regions(page, boxes, None, cx);
                continue;
            }
            let shape = match self.tool {
                Tool::Highlight => Shape::Highlight(boxes),
                Tool::Underline => Shape::Underline(boxes),
                _ => Shape::StrikeOut(boxes),
            };
            annots.push(NewAnnot { page, shape, rgb: None, contents: String::new(), author: String::new() });
        }
        self.selection = None;
        self.annotate("marking text", annots, cx);
    }

    /// Adds `annots` signed by the account's name.
    pub fn annotate(&mut self, what: &'static str, mut annots: Vec<NewAnnot>, cx: &mut Context<Self>) {
        if annots.is_empty() {
            return;
        }
        let author = ely_ui::settings::current(cx).author.clone();
        self.edit_at(what, move |worker, doc| {
            let name = ely_platform::user::named(&author)?;
            annots.iter_mut().for_each(|annot| annot.author = name.clone());
            worker.annotate(doc, &annots)
        }, cx);
    }

    /// A press with a drawing or form tool.
    pub fn sketch_from(&mut self, at: Point<Pixels>, cx: &mut Context<Self>) {
        let Some((page, point)) = self.page_at(at) else {
            return;
        };
        self.held = self.aim;
        self.grip(page);
        let hit = self.notes.get(&page).and_then(|marks| marks.fields.iter().rev().find(|f| inside(&f.rect, point)));
        (self.aim, self.sketch) = match (self.tool, hit) {
            (Tool::Note, _) => (Some(Aim::Note(Pick::NewNote { page, at: point })), None),
            (Tool::Stamp, _) => (Some(Aim::Note(Pick::NewStamp { page, at: point })), None),
            (Tool::Field(_), Some(field)) => (Some(Aim::Note(Pick::Field { page, id: field.id })), None),
            (Tool::Sign, Some(field)) if field.kind == FieldKind::Signature => {
                let object = u32::try_from(field.id).ok();
                self.sign_at = Some(crate::sign::SignAt { page, object, rect: field.rect, version: self.version });
                (None, None)
            }
            (Tool::Link, _) if let Some(index) = self.links.get(&page).and_then(|l| l.iter().rposition(|l| inside(&l.rect, point))) => {
                (Some(Aim::Note(Pick::Link { page, index })), None)
            }
            _ => (None, Some(Sketch { page, points: vec![point] })),
        };
        self.open_note_panel(cx);
        cx.notify();
    }

    pub fn sketch_to(&mut self, at: Point<Pixels>, cx: &mut Context<Self>) {
        let Some((page, point)) = self.held_at(at) else {
            return;
        };
        let ink = self.tool == Tool::Ink;
        if let Some(sketch) = self.sketch.as_mut().filter(|sketch| sketch.page == page) {
            match ink || sketch.points.len() < 2 {
                true => sketch.points.push(point),
                false => *sketch.points.last_mut().expect("two points") = point,
            }
            cx.notify();
        }
    }

    /// Ends a drawing: a shape lands, a field asks.
    pub fn sketch_end(&mut self, cx: &mut Context<Self>) {
        let Some(sketch) = self.sketch.take() else {
            return;
        };
        let (page, rect) = (sketch.page, sketch.rect());
        let small = rect[2] - rect[0] < 4.0 || rect[3] - rect[1] < 4.0;
        let first = sketch.points[0];
        let shape = match self.tool {
            Tool::Square if !small => Shape::Square(rect),
            Tool::Circle if !small => Shape::Circle(rect),
            Tool::Line if (rect[2] - rect[0]).max(rect[3] - rect[1]) >= 4.0 => {
                let last = sketch.points[sketch.points.len() - 1];
                Shape::Line([first.0, first.1, last.0, last.1])
            }
            Tool::Ink if sketch.points.len() > 1 => Shape::Ink(vec![sketch.points]),
            Tool::RedactArea => {
                let click = small.then_some(first);
                return self.mark_regions(page, if small { Vec::new() } else { vec![rect] }, click, cx);
            }
            Tool::PlaceSignature => return self.place_stamp(page, rect, small, cx),
            Tool::Sign => {
                let rect = if small { [first.0, first.1, first.0 + 200.0, first.1 + 60.0] } else { rect };
                self.sign_at = Some(crate::sign::SignAt { page, object: None, rect, version: self.version });
                return cx.notify();
            }
            Tool::Link => {
                let rect = if small { [first.0, first.1, first.0 + 120.0, first.1 + 20.0] } else { rect };
                self.aim = Some(Aim::Note(Pick::NewLink { page, rect }));
                self.open_note_panel(cx);
                return cx.notify();
            }
            Tool::Field(kind) => {
                let rect = match (small, kind) {
                    (false, _) => rect,
                    (true, FieldKind::Checkbox | FieldKind::Radio) => [first.0, first.1, first.0 + 14.0, first.1 + 14.0],
                    (true, _) => [first.0, first.1, first.0 + 150.0, first.1 + 20.0],
                };
                self.aim = Some(Aim::Note(Pick::NewField { page, rect, kind }));
                self.open_note_panel(cx);
                return cx.notify();
            }
            _ => return cx.notify(),
        };
        let annot = NewAnnot { page, shape, rgb: None, contents: String::new(), author: String::new() };
        self.annotate("drawing", vec![annot], cx);
    }

    /// The aimed annotation or field, and a shape being drawn.
    pub fn note_marks(&self, place: &Placed, scale: f32, cx: &App) -> Vec<AnyElement> {
        let color = cx.theme().colors.focus;
        let size = self.sizes[place.page];
        let outline = |rect: [f32; 4]| {
            let (x, y, w, h) = rect_on(rect, size, self.rotate, scale);
            div().absolute().left(px(x)).top(px(y)).w(px(w)).h(px(h)).border_1().border_color(color)
        };
        let mut marks = Vec::new();
        let aimed = match self.aim {
            Some(Aim::Note(Pick::Annot { page, id })) if page == place.page => {
                self.notes.get(&page).and_then(|m| m.annots.iter().find(|a| a.id == id)).map(|a| a.rect)
            }
            Some(Aim::Note(Pick::Field { page, id })) if page == place.page => {
                self.notes.get(&page).and_then(|m| m.fields.iter().find(|f| f.id == id)).map(|f| f.rect)
            }
            Some(Aim::Note(Pick::NewField { page, rect, .. } | Pick::NewLink { page, rect })) if page == place.page => Some(rect),
            Some(Aim::Note(Pick::Link { page, index })) if page == place.page => {
                self.links.get(&page).and_then(|links| links.get(index)).map(|l| l.rect)
            }
            _ => None,
        };
        marks.extend(aimed.map(|rect| outline(rect).into_any_element()));
        let Some(sketch) = self.sketch.as_ref().filter(|sketch| sketch.page == place.page) else {
            return marks;
        };
        match self.tool {
            Tool::Square | Tool::Field(_) | Tool::Link | Tool::RedactArea | Tool::Sign | Tool::PlaceSignature => marks.push(outline(sketch.rect()).into_any_element()),
            Tool::Circle => marks.push(outline(sketch.rect()).rounded_full().into_any_element()),
            _ => {
                let points: Vec<Point<Pixels>> = (sketch.points.iter())
                    .map(|p| to_view(*p, size, self.rotate))
                    .map(|(x, y)| point(px(x * scale), px(y * scale)))
                    .collect();
                marks.push(stroke(points, color).into_any_element());
            }
        }
        marks
    }
}

/// A line through `points`, over the page.
fn stroke(points: Vec<Point<Pixels>>, color: gpui::Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let mut path = gpui::PathBuilder::stroke(px(2.0));
            for (at, p) in points.iter().enumerate() {
                let p = bounds.origin + *p;
                if at == 0 { path.move_to(p) } else { path.line_to(p) }
            }
            match path.build() {
                Ok(path) => window.paint_path(path, color),
                Err(error) => log::error!("pdf: a stroke would not build: {error}"),
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ely_pdf_engine::Char;

    #[test]
    fn picked_text_marks_one_box_a_line_and_boxes_stay_on_the_page() {
        let c = |line, x: f32, y: f32| Char { line, c: 'a', rect: [x, y, x + 10.0, y + 12.0] };
        let chars = [c(0, 0.0, 0.0), c(0, 10.0, 1.0), c(1, 0.0, 20.0)];
        assert_eq!(line_boxes(&chars), [[0.0, 0.0, 20.0, 13.0], [0.0, 20.0, 10.0, 32.0]]);
        assert_eq!(around((5.0, 5.0), (20.0, 10.0), (100.0, 100.0)), [0.0, 0.0, 20.0, 10.0]);
        assert_eq!(around((50.0, 50.0), (20.0, 10.0), (100.0, 100.0)), [40.0, 45.0, 60.0, 55.0]);
    }
}
