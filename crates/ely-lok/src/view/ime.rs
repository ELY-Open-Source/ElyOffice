//! The IME composition bridge for LokView (LOK-07).

use std::ops::Range;

use gpui::{Bounds, Context, EntityInputHandler, Pixels, UTF16Selection, Window};

use super::{LokView, input::Input};

/// The live composition and its UTF-16 selection.
pub(super) struct Marked {
    pub text: String,
    pub selected: Range<usize>,
}

fn len16(text: &str) -> usize {
    text.encode_utf16().count()
}

/// The byte offset of a UTF-16 index, clamped.
fn byte_at(text: &str, at16: usize) -> usize {
    let mut seen = 0;
    for (byte, c) in text.char_indices() {
        if seen >= at16 {
            return byte;
        }
        seen += c.len_utf16();
    }
    text.len()
}

/// `base` with its UTF-16 `range` replaced by `text`.
fn splice(base: &str, range: &Range<usize>, text: &str) -> String {
    let (start, end) = (
        byte_at(base, range.start),
        byte_at(base, range.end.max(range.start)),
    );
    format!("{}{text}{}", &base[..start], &base[end..])
}

impl LokView {
    /// Replaces part of the composition; empty text cancels it.
    pub(super) fn compose(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
    ) {
        let (composed, start) = match (&self.marked, range) {
            (Some(marked), Some(range)) => (splice(&marked.text, &range, text), range.start),
            _ => (text.to_string(), 0),
        };
        self.send(Input::Text {
            text: composed.clone(),
            ends: composed.is_empty(),
        });
        if composed.is_empty() {
            self.marked = None;
            return;
        }
        let end = len16(&composed);
        let tail = start + len16(text);
        let selected = selected.map_or(tail..tail, |s| s.start + start..s.end + start);
        self.marked = Some(Marked {
            selected: selected.start.min(end)..selected.end.min(end),
            text: composed,
        });
    }

    /// Types `text` at the cursor.
    pub fn insert(&mut self, text: &str) {
        self.commit(None, text);
    }

    /// Commits `text` into the composition's `range`, or at the cursor.
    pub(super) fn commit(&mut self, range: Option<Range<usize>>, text: &str) {
        let committed = match (&self.marked, range) {
            (Some(marked), Some(range)) => splice(&marked.text, &range, text),
            _ => text.to_string(),
        };
        self.send(Input::Text {
            text: committed,
            ends: true,
        });
        self.marked = None;
    }
}

impl EntityInputHandler for LokView {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let marked = self.marked.as_ref()?;
        let end = len16(&marked.text);
        let range = range.start.min(end)..range.end.min(end);
        let text =
            &marked.text[byte_at(&marked.text, range.start)..byte_at(&marked.text, range.end)];
        *adjusted = Some(range);
        Some(text.to_string())
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let range = self
            .marked
            .as_ref()
            .map_or(0..0, |marked| marked.selected.clone());
        Some(UTF16Selection {
            range,
            reversed: false,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked.as_ref().map(|marked| 0..len16(&marked.text))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        if let Some(marked) = self.marked.take() {
            self.commit(None, &marked.text);
        }
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.commit(range, text);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.compose(range, text, selected);
    }

    fn bounds_for_range(
        &mut self,
        _: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        self.cursor_bounds()
    }

    fn character_index_for_point(
        &mut self,
        _: gpui::Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splices_count_utf16() {
        assert_eq!(splice("nihao", &(0..2), "你"), "你hao");
        assert_eq!(splice("你好", &(1..2), "们"), "你们");
        assert_eq!(splice("ab", &(5..9), "c"), "abc", "ranges clamp");
        assert_eq!(byte_at("😀a", 2), 4, "a surrogate pair is two units");
    }
}
