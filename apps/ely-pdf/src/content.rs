//! Text blocks and drawn objects, picked and dragged (PDF-03, PDF-04).

use std::collections::HashMap;

use ely_gpui_component::theme::ActiveTheme as _;
use ely_pdf_engine::{Drawn, TextBlock};
use gpui::{AnyElement, App, Context, IntoElement, Pixels, Point, Styled as _, div, px};

use crate::layout::Placed;
use crate::pages::rect_on;
use crate::view::{PdfView, Unread};

/// What a press on a page does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tool {
    #[default]
    Read,
    Edit,
    AddText,
    Highlight,
    Underline,
    StrikeOut,
    Note,
    Square,
    Circle,
    Line,
    Ink,
    Stamp,
    Field(ely_pdf_engine::FieldKind),
    Link,
    /// Marks picked text, or drawn areas, to remove.
    RedactText,
    RedactArea,
    /// Places a signature signed with a certificate.
    Sign,
    /// Places a visible signature made in its dialog.
    PlaceSignature,
}

impl Tool {
    /// Presses pick text, as in reading.
    pub fn selects(self) -> bool {
        matches!(self, Tool::Read | Tool::Highlight | Tool::Underline | Tool::StrikeOut | Tool::RedactText)
    }

    /// Presses aim at text blocks and objects.
    pub fn edits(self) -> bool {
        matches!(self, Tool::Edit | Tool::AddText)
    }
}

/// A page's text blocks and drawn objects.
#[derive(Clone, Debug, Default)]
pub struct Layer {
    pub blocks: Vec<TextBlock>,
    pub drawn: Vec<Drawn>,
}

/// What the edit panel works on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Aim {
    Text { page: usize, index: usize },
    Object { page: usize, index: usize },
    /// New text whose box starts here, in page points.
    New { page: usize, at: (f32, f32) },
    /// An annotation or form field.
    Note(crate::annotate::Pick),
}

/// An object being moved, or scaled from its corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drag {
    pub page: usize,
    pub index: usize,
    pub rect: [f32; 4],
    pub from: (f32, f32),
    pub now: (f32, f32),
    pub scale: bool,
    /// Where the press landed, in window pixels.
    pub pressed: (f32, f32),
    /// The pointer went past a click's slack.
    pub moved: bool,
}

/// The corner handle's reach, in page points.
const HANDLE: f32 = 6.0;

impl Drag {
    /// The page-space change the drag makes.
    pub fn change(&self) -> [f32; 6] {
        let ([x0, y0, x1, y1], (dx, dy)) = (self.rect, (self.now.0 - self.from.0, self.now.1 - self.from.1));
        match self.scale {
            false => [1.0, 0.0, 0.0, 1.0, dx, dy],
            true => {
                // Aspect kept, the top left fixed.
                let s = ((x1 - x0 + dx) / (x1 - x0)).max((y1 - y0 + dy) / (y1 - y0)).max(0.05);
                [s, 0.0, 0.0, s, x0 - s * x0, y0 - s * y0]
            }
        }
    }

    /// The object's box as the drag has it now.
    pub fn shown(&self) -> [f32; 4] {
        let [a, _, _, d, e, f] = self.change();
        let [x0, y0, x1, y1] = self.rect;
        [a * x0 + e, d * y0 + f, a * x1 + e, d * y1 + f]
    }
}

fn inside(rect: &[f32; 4], (x, y): (f32, f32)) -> bool {
    x >= rect[0] && x <= rect[2] && y >= rect[1] && y <= rect[3]
}

/// Whether `outer` holds all of `inner`.
fn covers(outer: &[f32; 4], inner: &[f32; 4]) -> bool {
    outer[0] <= inner[0] && outer[1] <= inner[1] && outer[2] >= inner[2] && outer[3] >= inner[3]
}

impl PdfView {
    /// A drag or drawing is under way.
    pub fn gesturing(&self) -> bool {
        self.drag.is_some() || self.sketch.is_some()
    }

    /// What the side panel shows; held still mid-gesture.
    pub fn panel_aim(&self) -> Option<Aim> {
        if self.gesturing() { self.held } else { self.aim }
    }

    /// A note's editor is the right panel: open it.
    pub fn open_note_panel(&mut self, cx: &mut Context<Self>) {
        if matches!(self.aim, Some(Aim::Note(_))) {
            ely_ui::settings::panels::show(false, true, cx);
        }
    }

    /// Changes the tool; what was aimed at is let go.
    pub fn set_tool(&mut self, tool: Tool, cx: &mut Context<Self>) {
        log::info!("pdf: the {tool:?} tool");
        (self.tool, self.aim, self.drag, self.sketch, self.selection, self.marking) = (tool, None, None, None, None, None);
        cx.notify();
    }

    /// Reads the layers of `pages`, one page at a time.
    pub fn ask_layers(&mut self, pages: &[usize], cx: &mut Context<Self>) {
        if !self.tool.edits() || self.reading_layer {
            return;
        }
        let Some(page) = (pages.iter().copied())
            .find(|page| !self.layers.contains_key(page) && !self.unread.contains(&Unread::Layer(*page)))
        else {
            return;
        };
        self.reading_layer = true;
        let (doc, version) = (self.doc, self.version);
        self.off(
            move |worker| Ok(Layer { blocks: worker.text_blocks(doc, page)?, drawn: worker.objects(doc, page)? }),
            move |view, layer, cx| {
                // Cleared either way; reads from before an edit retry.
                view.reading_layer = false;
                if view.version != version {
                    return cx.notify();
                }
                match layer {
                    Ok(layer) => {
                        view.layers.insert(page, layer);
                    }
                    Err(error) => {
                        view.unread.insert(Unread::Layer(page));
                        view.fail("reading what the page holds", &error, cx)
                    }
                }
                cx.notify();
            },
            cx,
        );
    }

    /// A press aims at an object, text, or new text.
    pub fn press(&mut self, at: Point<Pixels>, cx: &mut Context<Self>) {
        let Some((page, point)) = self.page_at(at) else {
            return;
        };
        (self.drag, self.held) = (None, self.aim);
        self.grip(page);
        if self.tool == Tool::AddText {
            self.aim = Some(Aim::New { page, at: point });
            return cx.notify();
        }
        let Some(layer) = self.layers.get(&page) else {
            return;
        };
        let corner = |rect: &[f32; 4]| (point.0 - rect[2]).abs() <= HANDLE && (point.1 - rect[3]).abs() <= HANDLE;
        let held = match self.aim {
            Some(Aim::Object { page: on, index }) if on == page => layer.drawn.get(index).filter(|d| corner(&d.rect)).map(|_| (index, true)),
            _ => None,
        };
        let text = layer.blocks.iter().position(|b| b.editable && inside(&b.rect, point));
        // Text wins over an object drawn all around it.
        let around = |rect: &[f32; 4]| text.is_some_and(|t| covers(rect, &layer.blocks[t].rect));
        let hit = held.or_else(|| {
            let object = layer.drawn.iter().rposition(|d| inside(&d.rect, point) && !around(&d.rect));
            object.map(|index| (index, false))
        });
        self.aim = match hit {
            Some((index, scale)) => {
                let rect = layer.drawn[index].rect;
                let pressed = (f32::from(at.x), f32::from(at.y));
                self.drag = Some(Drag { page, index, rect, from: point, now: point, scale, pressed, moved: false });
                Some(Aim::Object { page, index })
            }
            None => text.map(|index| Aim::Text { page, index }),
        };
        cx.notify();
    }

    pub fn drag_to(&mut self, at: Point<Pixels>, cx: &mut Context<Self>) {
        let Some((page, point)) = self.held_at(at) else {
            return;
        };
        if let Some(drag) = self.drag.as_mut().filter(|drag| drag.page == page) {
            let far = (f32::from(at.x) - drag.pressed.0).abs() + (f32::from(at.y) - drag.pressed.1).abs();
            drag.moved |= far >= crate::annotate::CLICK;
            drag.now = point;
            cx.notify();
        }
    }

    /// Ends a drag; a real move is one edit.
    pub fn release(&mut self, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        // A hand that shakes a little still clicks.
        if !drag.moved {
            return cx.notify();
        }
        // A queued edit may renumber what was dragged.
        if !self.settled() {
            let text = crate::viewer::Viewer::text(cx, "pdf.object.changed", &[]);
            cx.emit(crate::view::Failed(text.to_string()));
            return cx.notify();
        }
        let (page, index, change) = (drag.page, drag.index, drag.change());
        self.edit(
            if drag.scale { "scaling an object" } else { "moving an object" },
            move |worker, doc| worker.place(doc, page, index, change),
            cx,
        );
    }

    /// Outlines of what edits on `place`; the aimed one bold.
    pub fn edit_marks(&self, place: &Placed, scale: f32, cx: &App) -> Vec<AnyElement> {
        let Some(layer) = self.layers.get(&place.page).filter(|_| self.tool == Tool::Edit) else {
            return Vec::new();
        };
        let colors = &cx.theme().colors;
        let size = self.sizes[place.page];
        let outline = |rect: [f32; 4], strong: bool| {
            let (x, y, w, h) = rect_on(rect, size, self.rotate, scale);
            let color = if strong { colors.focus } else { colors.focus.opacity(0.35) };
            div().absolute().left(px(x)).top(px(y)).w(px(w)).h(px(h)).border_1().border_color(color).into_any_element()
        };
        let aimed = |object: bool, index: usize| match self.aim {
            Some(Aim::Object { page, index: at }) => object && page == place.page && at == index,
            Some(Aim::Text { page, index: at }) => !object && page == place.page && at == index,
            _ => false,
        };
        let mut marks: Vec<AnyElement> = (layer.blocks.iter().enumerate())
            .filter(|(_, b)| b.editable)
            .map(|(index, b)| outline(b.rect, aimed(false, index)))
            .collect();
        for (index, drawn) in layer.drawn.iter().enumerate() {
            let dragged = self.drag.filter(|d| d.page == place.page && d.index == index);
            let rect = dragged.map_or(drawn.rect, |d| d.shown());
            marks.push(outline(rect, aimed(true, index)));
            if aimed(true, index) {
                let (x, y, w, h) = rect_on(rect, size, self.rotate, scale);
                let corner = div().absolute().left(px(x + w - 4.0)).top(px(y + h - 4.0)).size(px(8.0)).bg(colors.focus);
                marks.push(corner.into_any_element());
            }
        }
        marks
    }
}

/// Layers by page, dropped with each edit.
pub type Layers = HashMap<usize, Layer>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drag_moves_or_scales_from_the_top_left() {
        let mut drag = Drag { page: 0, index: 0, rect: [10.0, 20.0, 110.0, 70.0], from: (50.0, 40.0), now: (60.0, 45.0), scale: false, pressed: (0.0, 0.0), moved: true };
        assert_eq!(drag.change(), [1.0, 0.0, 0.0, 1.0, 10.0, 5.0]);
        assert_eq!(drag.shown(), [20.0, 25.0, 120.0, 75.0]);
        drag = Drag { from: (110.0, 70.0), now: (210.0, 80.0), scale: true, ..drag };
        assert_eq!(drag.shown(), [10.0, 20.0, 210.0, 120.0], "twice as wide, twice as tall");
    }
}
