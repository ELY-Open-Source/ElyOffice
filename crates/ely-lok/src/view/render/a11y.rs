//! The paragraph as assistive tech reads it (UX-11).

use gpui::A11ySubtreeBuilder;
use gpui::accesskit::{Node, Role, TextPosition, TextSelection};

use crate::A11y;

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

/// The paragraph as one run; the caret on the view.
pub(super) fn paragraph(a11y: &A11y) -> impl FnOnce(&mut A11ySubtreeBuilder) + 'static {
    let (text, ends) = (a11y.text.clone(), ends(a11y));
    move |builder| {
        let id = builder.synthetic_node_id("paragraph");
        let mut run = Node::new(Role::TextRun);
        run.set_character_lengths(text.chars().map(|ch| ch.len_utf8() as u8).collect::<Vec<_>>());
        run.set_value(text);
        if !builder.push_child(id, run) {
            log::error!("lok: the paragraph's text run is a duplicate");
            return;
        }
        if let Some((anchor, focus)) = ends {
            let at = |character_index| TextPosition { node: id, character_index };
            builder.parent_node().set_text_selection(TextSelection {
                anchor: at(anchor),
                focus: at(focus),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{char_index, ends};
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
    fn the_selection_keeps_its_direction() {
        let a11y = |caret, selection| A11y { text: "a😀bc".into(), caret, selection };
        assert_eq!(ends(&a11y(None, None)), None);
        assert_eq!(ends(&a11y(Some(3), None)), Some((2, 2)));
        // LOK sends the anchor first, the caret side last.
        assert_eq!(ends(&a11y(Some(5), Some((1, 5)))), Some((1, 4)));
        assert_eq!(ends(&a11y(Some(1), Some((5, 1)))), Some((4, 1)));
    }
}
