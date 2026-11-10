//! The text as assistive tech reads it (UX-12).

use gpui::A11ySubtreeBuilder;
use gpui::accesskit::{Node, Role, TextPosition, TextSelection};

use crate::A11y;
use crate::view::LokView;

impl LokView {
    /// Names the view for assistive tech.
    pub fn set_name(&mut self, name: gpui::SharedString) {
        self.name = name;
    }
}

/// The char index at UTF-16 offset `at`.
fn char_index(text: &str, at: usize) -> usize {
    let mut units = 0;
    text.chars()
        .take_while(|ch| {
            let start = units;
            units += ch.len_utf16();
            start < at
        })
        .count()
}

/// Anchor and focus in chars; a caret alone is both.
fn ends(a11y: &A11y) -> Option<(usize, usize)> {
    let caret = a11y.caret?;
    let (anchor, focus) = a11y.selection.unwrap_or((caret, caret));
    Some((char_index(&a11y.text, anchor), char_index(&a11y.text, focus)))
}

/// A text run of `text`, one length per char.
fn run(text: String) -> Node {
    let mut run = Node::new(Role::TextRun);
    run.set_character_lengths(text.chars().map(|ch| ch.len_utf8() as u8).collect::<Vec<_>>());
    run.set_value(text);
    run
}

/// A run for each paragraph; the caret in its own.
pub(super) fn paragraphs(a11y: &A11y, body: Option<(&[String], usize)>) -> impl FnOnce(&mut A11ySubtreeBuilder) + 'static {
    let ((texts, caret), ends) = (lines(&a11y.text, body), ends(a11y));
    move |builder| {
        let mut held = None;
        for (index, text) in texts.into_iter().enumerate() {
            let id = builder.synthetic_node_id(("paragraph", index));
            if !builder.push_child(id, run(text)) {
                return log::error!("lok: paragraph {index} is a duplicate text run");
            }
            if index == caret {
                held = Some(id);
            }
        }
        if let (Some(node), Some((anchor, focus))) = (held, ends) {
            let at = |character_index| TextPosition { node, character_index };
            builder.parent_node().set_text_selection(TextSelection {
                anchor: at(anchor),
                focus: at(focus),
            });
        }
    }
}

/// Each run's text, and which holds the caret.
fn lines(caret: &str, body: Option<(&[String], usize)>) -> (Vec<String>, usize) {
    let Some((paragraphs, at)) = body else {
        return (vec![caret.to_string()], 0);
    };
    let last = paragraphs.len().saturating_sub(1);
    // The kernel's own event has the caret's paragraph as typed.
    let texts = paragraphs.iter().enumerate().map(|(index, text)| (index, if index == at { caret } else { text }));
    (texts.map(|(index, text)| if index < last { format!("{text}\n") } else { text.to_string() }).collect(), at)
}

#[cfg(test)]
mod tests {
    use super::{char_index, ends, lines};
    use crate::A11y;

    #[test]
    fn utf16_offsets_land_on_chars() {
        // The emoji is two UTF-16 units, one char.
        assert_eq!(char_index("a😀b", 0), 0);
        assert_eq!(char_index("a😀b", 1), 1);
        assert_eq!(char_index("a😀b", 3), 2);
        assert_eq!(char_index("a😀b", 4), 3);
        assert_eq!(char_index("你好", 9), 2, "past the end is the end");
    }

    #[test]
    fn the_body_reads_line_by_line_around_the_caret_s_paragraph() {
        assert_eq!(lines("only", None), (vec!["only".to_string()], 0), "no body: the paragraph alone, exact");
        let body = ["one".to_string(), "stale".to_string(), String::new(), "last".to_string()];
        let (texts, caret) = lines("typed", Some((&body, 1)));
        assert_eq!((texts, caret), (vec!["one\n".to_string(), "typed\n".into(), "\n".into(), "last".into()], 1));
    }

    #[test]
    fn the_selection_keeps_its_direction() {
        let a11y = |caret, selection| A11y { text: "a😀bc".into(), caret, selection, focused: 0 };
        assert_eq!(ends(&a11y(None, None)), None);
        assert_eq!(ends(&a11y(Some(3), None)), Some((2, 2)));
        // LOK sends the anchor first, the caret side last.
        assert_eq!(ends(&a11y(Some(5), Some((1, 5)))), Some((1, 4)));
        assert_eq!(ends(&a11y(Some(1), Some((5, 1)))), Some((4, 1)));
    }
}
