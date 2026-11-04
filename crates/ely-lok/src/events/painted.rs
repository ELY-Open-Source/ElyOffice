//! The kernel's painted box: invalidations outside it never come.

use std::collections::HashMap;

use super::{Rect, slots};

/// CallbackFlushHandler's painted tiles, and the view's visible area.
#[derive(Default)]
pub(super) struct Painted {
    /// By part and edit mode: left, top, right, bottom.
    boxes: HashMap<(i32, i32), (i32, i32, i32, i32)>,
    /// The visible area the view last set.
    visible: Option<Rect>,
}

impl Painted {
    /// An EMPTY invalidation clears its part's box.
    pub(super) fn emptied(&mut self, text: &str) {
        match at_of(text) {
            Some(at) => _ = self.boxes.remove(&at),
            // Unread: assume every box cleared.
            None => self.boxes.clear(),
        }
    }
}

/// "EMPTY, part, mode" as part and mode.
fn at_of(text: &str) -> Option<(i32, i32)> {
    let mut fields = text.strip_prefix("EMPTY,")?.split(',').map(|field| field.trim().parse().ok());
    let at = (fields.next()??, fields.next()??);
    fields.next().is_none().then_some(at)
}

/// The kernel painted `rect` of part and mode `at`.
pub fn painted(id: u64, at: (i32, i32), (x, y, w, h): Rect) {
    let mut slots = slots().lock().expect("the events lock");
    let Some(slot) = slots.get_mut(&id) else { return };
    let grown = (x, y, x.saturating_add(w), y.saturating_add(h));
    slot.painted.boxes.entry(at).and_modify(|bbox| *bbox = union(*bbox, grown)).or_insert(grown);
}

/// The view shows `rect` of the document from now on.
pub fn shown(id: u64, rect: Rect) {
    if let Some(slot) = slots().lock().expect("the events lock").get_mut(&id) {
        slot.painted.visible = Some(rect);
    }
}

/// Whether the kernel may drop invalidations the view shows.
pub fn blind(id: u64, at: (i32, i32), clip: Option<Rect>) -> bool {
    let slots = slots().lock().expect("the events lock");
    let Some(painted) = slots.get(&id).map(|slot| &slot.painted) else { return true };
    let Some((x, y, w, h)) = painted.visible else { return true };
    let mut shown = (x, y, x.saturating_add(w), y.saturating_add(h));
    if let Some((cx, cy, cw, ch)) = clip {
        shown = (shown.0.max(cx), shown.1.max(cy), shown.2.min(cx.saturating_add(cw)), shown.3.min(cy.saturating_add(ch)));
    }
    if shown.0 >= shown.2 || shown.1 >= shown.3 {
        return false;
    }
    painted.boxes.get(&at).is_none_or(|bbox| bbox.0 > shown.0 || bbox.1 > shown.1 || bbox.2 < shown.2 || bbox.3 < shown.3)
}

fn union(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) {
    (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_box_follows_paints_and_empty_invalidations() {
        let mut painted = Painted::default();
        painted.boxes.insert((0, 0), (0, 0, 10, 10));
        painted.boxes.insert((1, 0), (0, 0, 10, 10));
        painted.emptied("EMPTY, 1, 0");
        assert_eq!(painted.boxes.keys().collect::<Vec<_>>(), [&(0, 0)], "only its part clears");
        painted.emptied("EMPTY, -1, 0");
        assert_eq!(painted.boxes.len(), 1, "the all-parts box is its own");
        painted.emptied("EMPTY");
        assert!(painted.boxes.is_empty(), "an unread one clears all");
        assert_eq!(at_of("EMPTY, 2, 1"), Some((2, 1)));
        assert_eq!(at_of("EMPTY, 2, 1, 4"), None);
        assert_eq!(union((0, 0, 3840, 3840), (3840, 0, 7680, 3840)), (0, 0, 7680, 3840));
    }
}
