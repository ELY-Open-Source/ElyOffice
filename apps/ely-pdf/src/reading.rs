//! Shown pages as assistive tech reads them (UX-11).

use std::{ops::Range, sync::Arc};

use ely_pdf_engine::Char;
use gpui::A11ySubtreeBuilder;
use gpui::accesskit::{Node, Role, TextPosition, TextSelection};

use crate::select::Span;

/// One text line: its page, first char index, and characters.
struct Line {
    page: usize,
    first: usize,
    chars: Vec<char>,
}

/// A page's lines in order.
fn lines(page: usize, chars: &[Char]) -> Vec<Line> {
    let mut first = 0;
    chars
        .chunk_by(|a, b| a.line == b.line)
        .map(|line| {
            let made = Line { page, first, chars: line.iter().map(|c| c.c).collect() };
            first += line.len();
            made
        })
        .collect()
}

/// The part of `line` the span picks, if any.
fn picked(line: &Line, span: Span) -> Option<Range<usize>> {
    let last = line.first + line.chars.len().checked_sub(1)?;
    if (line.page, last) < span.from || (line.page, line.first) > span.to {
        return None;
    }
    let from = span.from.max((line.page, line.first)).1 - line.first;
    let to = span.to.min((line.page, last)).1 - line.first;
    Some(from..to + 1)
}

/// Shown lines as text runs; the picked span as selection.
pub fn pages(shown: Vec<(usize, Arc<Vec<Char>>)>, span: Option<Span>) -> impl FnOnce(&mut A11ySubtreeBuilder) + 'static {
    move |builder| {
        let (mut anchor, mut focus) = (None, None);
        for (page, chars) in &shown {
            for line in lines(*page, chars) {
                let id = builder.synthetic_node_id((line.page, line.first));
                // Ends off screen clip to the shown lines.
                if let Some(range) = span.and_then(|span| picked(&line, span)) {
                    anchor = anchor.or(Some(TextPosition { node: id, character_index: range.start }));
                    focus = Some(TextPosition { node: id, character_index: range.end });
                }
                let mut run = Node::new(Role::TextRun);
                let mut lengths: Vec<u8> = line.chars.iter().map(|c| c.len_utf8() as u8).collect();
                lengths.push(1);
                run.set_character_lengths(lengths);
                run.set_value(line.chars.iter().chain(['\n'].iter()).collect::<String>());
                if !builder.push_child(id, run) {
                    log::error!("pdf: page {} line at {} is a duplicate run", line.page, line.first);
                }
            }
        }
        if let (Some(anchor), Some(focus)) = (anchor, focus) {
            builder.parent_node().set_text_selection(TextSelection { anchor, focus });
        }
    }
}

#[cfg(test)]
mod tests {
    use ely_pdf_engine::Char;

    use super::{lines, picked};
    use crate::select::Span;

    fn chars(text: &str) -> Vec<Char> {
        let mut line = 0;
        text.chars()
            .filter_map(|c| {
                if c == '|' {
                    line += 1;
                    return None;
                }
                Some(Char { line, c, rect: [0.0; 4] })
            })
            .collect()
    }

    #[test]
    fn a_page_reads_line_by_line() {
        let read = lines(2, &chars("Hello|PDF 世界"));
        let text: Vec<String> = read.iter().map(|line| line.chars.iter().collect()).collect();
        assert_eq!(text, ["Hello", "PDF 世界"]);
        assert_eq!(read[1].first, 5, "the second line starts after five chars");
        let span = |from, to| Span { from, to };
        assert_eq!(picked(&read[1], span((2, 6), (2, 7))), Some(1..3));
        assert_eq!(picked(&read[1], span((2, 0), (2, 4))), None, "the line before");
        // An end on a page off screen clips.
        assert_eq!(picked(&read[1], span((1, 3), (2, 6))), Some(0..2));
        assert_eq!(picked(&read[1], span((2, 7), (5, 0))), Some(2..6));
        assert_eq!(picked(&read[0], span((1, 0), (3, 0))), Some(0..5), "inside both ends");
        assert_eq!(picked(&read[1], span((3, 0), (4, 0))), None, "pages after");
    }
}
