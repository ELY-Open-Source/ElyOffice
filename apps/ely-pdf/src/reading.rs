//! Shown pages as assistive tech reads them (UX-11).

use std::sync::Arc;

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

/// Where `(page, index)` falls in `line`, if it does.
fn within(line: &Line, (page, index): (usize, usize)) -> Option<usize> {
    let offset = index.checked_sub(line.first)?;
    (page == line.page && offset < line.chars.len()).then_some(offset)
}

/// Shown lines as text runs; the picked span as selection.
pub fn pages(shown: Vec<(usize, Arc<Vec<Char>>)>, span: Option<Span>) -> impl FnOnce(&mut A11ySubtreeBuilder) + 'static {
    move |builder| {
        let (mut anchor, mut focus) = (None, None);
        for (page, chars) in &shown {
            for line in lines(*page, chars) {
                let id = builder.synthetic_node_id((line.page, line.first));
                if let Some(span) = span {
                    anchor = anchor.or(within(&line, span.from).map(|at| TextPosition { node: id, character_index: at }));
                    // The span's end is inclusive.
                    focus = focus.or(within(&line, span.to).map(|at| TextPosition { node: id, character_index: at + 1 }));
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

    use super::{lines, within};

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
        assert_eq!(within(&read[1], (2, 6)), Some(1));
        assert_eq!(within(&read[1], (2, 4)), None, "on the line before");
        assert_eq!(within(&read[1], (3, 6)), None, "another page");
    }
}
