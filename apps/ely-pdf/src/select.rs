//! Text picked with the mouse, across pages, and copied (PDF-01).

use std::sync::Arc;

use ely_gpui_component::theme::ActiveTheme as _;
use ely_pdf_engine::Char;
use gpui::{AnyElement, App, ClipboardItem, Context, IntoElement, Pixels, Point, Styled as _, div, px};

use crate::layout::Placed;
use crate::pages::rect_on;
use crate::search::bounds;
use crate::view::PdfView;

/// Drag start and end, as (page, point in points).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Selection {
    pub anchor: (usize, (f32, f32)),
    pub head: (usize, (f32, f32)),
}

/// Picked characters from one (page, index) to another.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub from: (usize, usize),
    pub to: (usize, usize),
}

impl Selection {
    /// The characters picked, once both ends' pages are read.
    pub fn span<'a>(&self, text: impl Fn(usize) -> Option<&'a [Char]>) -> Option<Span> {
        // A press that has not moved picks nothing.
        if self.anchor == self.head {
            return None;
        }
        let end = |(page, point): (usize, (f32, f32))| Some((page, nearest(text(page)?, point)?));
        let (a, b) = (end(self.anchor)?, end(self.head)?);
        Some(Span { from: a.min(b), to: a.max(b) })
    }
}

impl Span {
    /// The picked characters of `page`, if any.
    pub fn on(&self, page: usize, count: usize) -> Option<std::ops::Range<usize>> {
        let ((p0, i0), (p1, i1)) = (self.from, self.to);
        if page < p0 || page > p1 || count == 0 {
            return None;
        }
        let from = if page == p0 { i0 } else { 0 };
        let to = if page == p1 { i1 + 1 } else { count };
        Some(from.min(count)..to.min(count))
    }
}

/// The character nearest `at`, in page points.
pub fn nearest(chars: &[Char], (x, y): (f32, f32)) -> Option<usize> {
    let gap = |c: &Char| {
        let dx = (c.rect[0] - x).max(0.0).max(x - c.rect[2]);
        let dy = (c.rect[1] - y).max(0.0).max(y - c.rect[3]);
        // Lines count more than columns.
        dx + dy * 4.0
    };
    (0..chars.len()).min_by(|a, b| gap(&chars[*a]).total_cmp(&gap(&chars[*b])))
}

/// Characters as text: a line change is a new line.
pub fn as_text(chars: &[Char]) -> String {
    let mut text = String::new();
    for (at, c) in chars.iter().enumerate() {
        if at > 0 && chars[at - 1].line != c.line {
            text.push('\n');
        }
        text.push(c.c);
    }
    text
}

impl PdfView {
    /// The text of `page`, asked for when missing.
    pub fn page_text(&mut self, page: usize, cx: &mut Context<Self>) -> Option<Arc<Vec<Char>>> {
        if let Some(text) = self.text.get(&page) {
            return Some(text.clone());
        }
        if self.asking_text.insert(page) {
            let doc = self.doc;
            self.read(
                move |worker| worker.text(doc, page),
                move |view, read, cx| {
                    view.asking_text.remove(&page);
                    match read {
                        Ok(chars) => {
                            view.text.insert(page, Arc::new(chars));
                            view.mark_pending(cx);
                            cx.notify();
                        }
                        Err(error) => {
                            view.marking = None;
                            view.fail("pdf.doing.text", &error, cx)
                        }
                    }
                },
                cx,
            );
        }
        None
    }

    /// The page point under `at`; its text is asked for.
    fn point_at(&mut self, at: Point<Pixels>, cx: &mut Context<Self>) -> Option<(usize, (f32, f32))> {
        let (page, point) = self.page_at(at)?;
        self.page_text(page, cx);
        Some((page, point))
    }

    pub fn select_from(&mut self, at: Point<Pixels>, cx: &mut Context<Self>) {
        (self.marking, self.pressed) = (None, Some(at));
        self.selection = self.point_at(at, cx).map(|end| Selection { anchor: end, head: end });
        cx.notify();
    }

    pub fn select_to(&mut self, at: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(head) = self.point_at(at, cx) else {
            return;
        };
        if let Some(selection) = self.selection.as_mut()
            && selection.head != head
        {
            selection.head = head;
            cx.notify();
        }
    }

    /// Picked text and found words drawn over a page.
    pub fn page_marks(&self, place: &Placed, scale: f32, cx: &App) -> Vec<AnyElement> {
        let colors = &cx.theme().colors;
        let size = self.sizes[place.page];
        let mark = |rect: [f32; 4], color| {
            let (x, y, w, h) = rect_on(rect, size, self.rotate, scale);
            div().absolute().left(px(x)).top(px(y)).w(px(w)).h(px(h)).bg(color).into_any_element()
        };
        let mut marks = Vec::new();
        let span = self.selection.and_then(|selection| selection.span(|page| self.text.get(&page).map(|chars| chars.as_slice())));
        if let (Some(span), Some(chars)) = (span, self.text.get(&place.page)) {
            for c in span.on(place.page, chars.len()).map_or(&[][..], |range| &chars[range]) {
                marks.push(mark(c.rect, colors.selection));
            }
        }
        for (at, (_, quads)) in self.found.iter().enumerate().filter(|(_, (page, _))| *page == place.page) {
            let color = match self.chosen == Some(at) {
                true => colors.warning.opacity(0.5),
                false => colors.warning.opacity(0.25),
            };
            marks.extend(quads.iter().map(|quad| mark(bounds(quad), color)));
        }
        marks
    }

    /// Copies the picked text, reading missing pages first.
    pub fn copy(&mut self, cx: &mut Context<Self>) {
        let Some(selection) = self.selection else {
            return;
        };
        if self.allowed & ely_pdf_engine::proto::may::COPY == 0 {
            return cx.emit(crate::view::Failed(crate::view::restricted("copy", cx).to_string()));
        }
        let (first, last) = (selection.anchor.0.min(selection.head.0), selection.anchor.0.max(selection.head.0));
        let (doc, have) = (self.doc, self.text.clone());
        self.read(
            move |worker| {
                (first..=last)
                    .map(|page| match have.get(&page) {
                        Some(chars) => Ok((**chars).clone()),
                        None => worker.text(doc, page),
                    })
                    .collect::<std::io::Result<Vec<_>>>()
            },
            move |view, read, cx| match read {
                Ok(pages) => {
                    let Some(span) = selection.span(|page| pages.get(page.checked_sub(first)?).map(Vec::as_slice)) else {
                        return log::info!("pdf: nothing picked to copy");
                    };
                    let parts: Vec<String> = (pages.iter().enumerate())
                        .filter_map(|(at, chars)| {
                            let range = span.on(first + at, chars.len())?;
                            Some(as_text(&chars[range]))
                        })
                        .collect();
                    log::info!("pdf: copies text from {} pages", parts.len());
                    cx.write_to_clipboard(ClipboardItem::new_string(parts.join("\n")));
                }
                Err(error) => view.fail("pdf.doing.copy", &error, cx),
            },
            cx,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(line: i32, text: &str, y: f32) -> Vec<Char> {
        (text.chars().enumerate())
            .map(|(at, c)| Char { line, c, rect: [at as f32 * 10.0, y, at as f32 * 10.0 + 10.0, y + 12.0] })
            .collect()
    }

    #[test]
    fn a_drag_picks_from_either_end_and_copies_lines() {
        let mut chars = line(0, "Hello", 0.0);
        chars.extend(line(1, "World", 20.0));
        assert_eq!(nearest(&chars, (25.0, 5.0)), Some(2), "inside the third");
        assert_eq!(nearest(&chars, (500.0, 25.0)), Some(9), "past the end of line two");
        let text = |page: usize| (page == 0).then_some(chars.as_slice());
        let backwards = Selection { anchor: (0, (25.0, 25.0)), head: (0, (25.0, 5.0)) };
        let span = backwards.span(text).expect("both ends read");
        assert_eq!(as_text(&chars[span.on(0, chars.len()).expect("on page zero")]), "llo\nWor");
        let still = Selection { anchor: (0, (25.0, 5.0)), head: (0, (25.0, 5.0)) };
        assert_eq!(still.span(text), None, "a click picks nothing");
        let unread = Selection { anchor: (0, (25.0, 5.0)), head: (1, (5.0, 5.0)) };
        assert_eq!(unread.span(text), None, "page two is not read yet");
        let across = Span { from: (1, 3), to: (3, 1) };
        assert_eq!((across.on(1, 5), across.on(2, 5), across.on(3, 5), across.on(4, 5)), (Some(3..5), Some(0..5), Some(0..2), None));
    }
}
