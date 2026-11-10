// Copyright 2022 The AccessKit Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0 (found in
// the LICENSE-APACHE file) or the MIT license (found in
// the LICENSE-MIT file), at your option.

use accesskit::{Point, Rect, ScrollHint};
use accesskit_consumer::{Node, TextPosition, TextRange};
use atspi_common::{CoordType, Granularity, ScrollType};

use crate::Error;

#[derive(Clone, Copy, Default, Debug)]
pub struct WindowBounds {
    pub outer: Rect,
    pub inner: Rect,
}

impl WindowBounds {
    pub fn new(outer: Rect, inner: Rect) -> Self {
        Self { outer, inner }
    }

    pub(crate) fn accesskit_point_to_atspi_point(
        &self,
        point: Point,
        parent: Option<Node>,
        coord_type: CoordType,
    ) -> Point {
        let origin = self.origin(parent, coord_type);
        Point::new(origin.x + point.x, origin.y + point.y)
    }

    pub(crate) fn atspi_point_to_accesskit_point(
        &self,
        point: Point,
        parent: Option<Node>,
        coord_type: CoordType,
    ) -> Point {
        let origin = self.origin(parent, coord_type);
        Point::new(point.x - origin.x, point.y - origin.y)
    }

    fn origin(&self, parent: Option<Node>, coord_type: CoordType) -> Point {
        match coord_type {
            CoordType::Screen => self.inner.origin(),
            CoordType::Window => Point::ZERO,
            CoordType::Parent => {
                if let Some(parent) = parent {
                    let parent_origin = parent.bounding_box().unwrap_or_default().origin();
                    Point::new(-parent_origin.x, -parent_origin.y)
                } else {
                    self.inner.origin()
                }
            }
        }
    }
}

pub(crate) fn text_position_from_offset<'a>(
    node: &'a Node,
    offset: i32,
) -> Option<TextPosition<'a>> {
    let index = offset.try_into().ok()?;
    node.text_position_from_global_usv_index(index)
}

/// Whether `mark` can end a sentence.
fn is_sentence_mark(mark: char) -> bool {
    matches!(mark, '.' | '!' | '?' | '。' | '！' | '？')
}

/// Whether `mark`, right after a sentence mark, closes a quotation or
/// a bracket. German closes with “ and «, so no space decides it.
fn is_closer(mark: char) -> bool {
    matches!(
        mark,
        '"' | '\'' | '”' | '’' | '“' | '‘' | '»' | '«' | '›' | '‹' | ')' | ']' | '」' | '』' | '）' | '】' | '》'
    )
}

/// The end of the sentence marks and closers that start at `from`.
/// French sets a no-break space before its closing » and ›.
fn closed(text: &[char], mut from: usize) -> usize {
    loop {
        match text[from..] {
            [mark, ..] if is_sentence_mark(mark) || is_closer(mark) => from += 1,
            ['\u{a0}' | '\u{202f}', '»' | '›', ..] => from += 2,
            _ => return from,
        }
    }
}

/// The sentence around char `at`: its start and its end, in chars.
///
/// A CJK full stop ends a sentence anywhere; a Latin one only when
/// whitespace or the end follows it and any closing quotes. Those
/// quotes and the whitespace after belong to the sentence.
fn sentence_bounds(text: &[char], at: usize) -> (usize, usize) {
    let closed = |from: usize| closed(text, from);
    let ends = |index: usize| match text[index] {
        '。' | '！' | '？' => true,
        '.' | '!' | '?' => text
            .get(closed(index + 1))
            .is_none_or(|next| next.is_whitespace()),
        _ => false,
    };
    let (mut start, mut index) = (0, 0);
    while index < text.len() {
        if !ends(index) {
            index += 1;
            continue;
        }
        // An opening quote after the space starts the next one.
        let mut next = closed(index + 1);
        while next < text.len() && text[next].is_whitespace() {
            next += 1;
        }
        if at < next {
            return (start, next);
        }
        (start, index) = (next, next);
    }
    (start, text.len())
}

/// The sentence at `offset`, found inside its paragraph.
fn sentence_range_from_offset<'a>(
    node: &'a Node,
    offset: TextPosition<'a>,
) -> Result<TextRange<'a>, Error> {
    let first = match offset.is_paragraph_start() {
        true => offset,
        false => offset.backward_to_paragraph_start(),
    };
    let mut paragraph = first.to_degenerate_range();
    paragraph.set_end(first.forward_to_paragraph_end());
    let text: Vec<char> = paragraph.text().chars().collect();
    let base = first.to_global_usv_index();
    let (start, end) = sentence_bounds(&text, offset.to_global_usv_index() - base);
    let at = |index: usize| {
        node.text_position_from_global_usv_index(base + index)
            .ok_or(Error::IndexOutOfRange)
    };
    let mut range = at(start)?.to_degenerate_range();
    range.set_end(at(end)?);
    Ok(range)
}

pub(crate) fn text_range_from_offset<'a>(
    node: &'a Node,
    offset: i32,
    granularity: Granularity,
) -> Result<TextRange<'a>, Error> {
    let start_offset = text_position_from_offset(node, offset).ok_or(Error::IndexOutOfRange)?;
    let start = match granularity {
        Granularity::Char => start_offset,
        Granularity::Line if start_offset.is_line_start() => start_offset,
        Granularity::Line => start_offset.backward_to_line_start(),
        Granularity::Paragraph if start_offset.is_paragraph_start() => start_offset,
        Granularity::Paragraph => start_offset.backward_to_paragraph_start(),
        Granularity::Sentence => sentence_range_from_offset(node, start_offset)?.start(),
        Granularity::Word if start_offset.is_word_start() => start_offset,
        Granularity::Word => start_offset.backward_to_word_start(),
    };
    let end = match granularity {
        Granularity::Char if start_offset.is_document_end() => start_offset,
        Granularity::Char => start.forward_to_character_end(),
        Granularity::Line => start.forward_to_line_end(),
        Granularity::Paragraph => start.forward_to_paragraph_end(),
        Granularity::Sentence => sentence_range_from_offset(node, start_offset)?.end(),
        Granularity::Word => start.forward_to_word_end(),
    };
    let mut range = start.to_degenerate_range();
    range.set_end(end);
    Ok(range)
}

pub(crate) fn text_range_from_offsets<'a>(
    node: &'a Node,
    start_offset: i32,
    end_offset: i32,
) -> Option<TextRange<'a>> {
    let start = text_position_from_offset(node, start_offset)?;
    let end = if end_offset == -1 {
        node.document_range().end()
    } else {
        text_position_from_offset(node, end_offset)?
    };

    let mut range = start.to_degenerate_range();
    range.set_end(end);
    Some(range)
}

pub(crate) fn text_range_bounds_from_offsets(
    node: &Node,
    start_offset: i32,
    end_offset: i32,
) -> Option<Rect> {
    text_range_from_offsets(node, start_offset, end_offset)?
        .bounding_boxes()
        .into_iter()
        .reduce(|rect1, rect2| rect1.union(rect2))
}

pub(crate) fn atspi_scroll_type_to_scroll_hint(scroll_type: ScrollType) -> Option<ScrollHint> {
    match scroll_type {
        ScrollType::TopLeft => Some(ScrollHint::TopLeft),
        ScrollType::BottomRight => Some(ScrollHint::BottomRight),
        ScrollType::TopEdge => Some(ScrollHint::TopEdge),
        ScrollType::BottomEdge => Some(ScrollHint::BottomEdge),
        ScrollType::LeftEdge => Some(ScrollHint::LeftEdge),
        ScrollType::RightEdge => Some(ScrollHint::RightEdge),
        ScrollType::Anywhere => None,
    }
}

#[cfg(test)]
mod tests {
    use super::sentence_bounds;

    fn bounds(text: &str, at: usize) -> (usize, usize) {
        sentence_bounds(&text.chars().collect::<Vec<_>>(), at)
    }

    #[test]
    fn a_sentence_runs_to_the_next_one_s_start() {
        let text = "One. Two 2.5 now!  Three\n";
        assert_eq!(bounds(text, 0), (0, 5));
        assert_eq!(bounds(text, 4), (0, 5), "the space after belongs to it");
        assert_eq!(bounds(text, 5), (5, 19), "a point inside a number ends nothing");
        assert_eq!(bounds(text, 19), (19, 25), "the last runs to the end");
        assert_eq!(bounds("一。二？！三", 2), (2, 5), "a CJK stop needs no space");
        assert_eq!(bounds("一。二？！三", 5), (5, 6));
        assert_eq!(bounds("no stop", 3), (0, 7));
        let quoted = "He said \"Go.\" Next.";
        assert_eq!(bounds(quoted, 16), (14, 19), "a closing quote ends with its sentence");
        assert_eq!(bounds(quoted, 12), (0, 14));
        assert_eq!(bounds("「一。」二。", 4), (4, 6), "a CJK closing bracket too");
        assert_eq!(bounds("Go. \"Next\" one.", 5), (4, 15), "an opening quote starts the next");
        assert_eq!(bounds("(See 2.) Then", 9), (9, 13));
        let german = "Er sagte: „Hallo.“ Weiter.";
        assert_eq!(bounds(german, 20), (19, 26), "German closes with the upper quote");
        assert_eq!(bounds("»Halt.« Dann", 9), (8, 12));
        let french = "Il dit : «\u{a0}Oui.\u{a0}» Ensuite.";
        assert_eq!(bounds(french, 20), (18, 26), "French sets a space before its closer");
        assert_eq!(bounds(french, 12), (0, 18));
        assert_eq!(bounds("Oui.\u{a0}Non.", 6), (5, 9), "a no-break space alone ends it");
        assert_eq!(bounds("", 0), (0, 0));
    }
}
