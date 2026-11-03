//! Each edit, timed to the screen that shows it (PERF-02).
use std::{collections::VecDeque, time::Instant};

/// The generation showing an edit; None: no tile changed.
pub(super) type Picture = Result<Option<u64>, &'static str>;

/// An edit's picture from the generations read around it.
pub(super) fn picture(before: Option<(u64, u64)>, after: Option<(u64, u64)>, failed: bool) -> Picture {
    match (before, after) {
        _ if failed => Err("it failed"),
        (Some((_, before)), Some((_, after))) => Ok((after > before).then_some(after)),
        _ => Err("its state is unread"),
    }
}

/// One edit on its way to the screen.
struct Edit {
    seq: u64,
    kind: &'static str,
    at: Instant,
    /// The generation that shows it, once it ran.
    shows: Option<u64>,
}

#[derive(Default)]
pub(super) struct EditClock {
    /// Edits not yet on screen, oldest first.
    edits: VecDeque<Edit>,
    /// Whether the view draws; a hidden view's edits never show.
    shown: bool,
}

impl EditClock {
    /// Edit `seq` of `kind` was queued.
    pub(super) fn sent(&mut self, seq: u64, kind: &'static str) {
        if !self.shown {
            log::debug!("lok: edit {seq} {kind} never drawn: the view is hidden");
            return;
        }
        self.edits.push_back(Edit { seq, kind, at: Instant::now(), shows: None });
    }

    /// Edit `seq` ran in the kernel with `picture`.
    pub(super) fn landed(&mut self, seq: u64, picture: Picture) {
        let Some(at) = self.edits.iter().position(|edit| edit.seq == seq) else {
            return;
        };
        match picture {
            Ok(Some(generation)) => self.edits[at].shows = Some(generation),
            Ok(None) => self.never(at, "it changed no tile"),
            Err(why) => self.never(at, why),
        }
    }

    /// A whole screen at `generation` reached the window.
    pub(super) fn drawn(&mut self, generation: u64) {
        self.edits.retain(|edit| {
            let shown = edit.shows.is_some_and(|shows| shows <= generation);
            if shown {
                log::debug!("lok: edit {} {} to tile {} ms", edit.seq, edit.kind, edit.at.elapsed().as_millis());
            }
            !shown
        });
    }

    /// The view draws from now on.
    pub(super) fn showing(&mut self) {
        self.shown = true;
    }

    /// The view stopped drawing; its edits never show.
    pub(super) fn hidden(&mut self) {
        self.shown = false;
        while !self.edits.is_empty() {
            self.never(0, "the view was hidden");
        }
    }

    /// Whether an edit waits for its screen.
    pub(super) fn waiting(&self) -> bool {
        !self.edits.is_empty()
    }

    fn never(&mut self, at: usize, why: &str) {
        if let Some(edit) = self.edits.remove(at) {
            log::debug!("lok: edit {} {} never drawn: {why}", edit.seq, edit.kind);
        }
    }

    #[cfg(test)]
    fn pending(&self) -> Vec<u64> {
        self.edits.iter().map(|edit| edit.seq).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{EditClock, picture};

    fn shown() -> EditClock {
        let mut clock = EditClock::default();
        clock.showing();
        clock
    }

    #[test]
    fn a_recovery_point_between_edit_and_tile_still_pairs_them() {
        let mut clock = shown();
        clock.sent(1, "text");
        clock.landed(1, picture(Some((7, 2)), Some((8, 3)), false));
        // The point's repaint stales the screen; older wholes show.
        clock.drawn(2);
        assert_eq!(clock.pending(), [1], "a screen from before the edit");
        clock.drawn(4);
        assert!(clock.pending().is_empty(), "the screen after the point shows it");
        // A key that changed nothing is not timed by it.
        clock.sent(2, "key");
        clock.landed(2, picture(Some((8, 4)), Some((9, 4)), false));
        clock.drawn(5);
        assert!(clock.pending().is_empty());
    }

    #[test]
    fn a_deck_still_opening_does_not_stop_the_keys_being_timed() {
        let mut clock = shown();
        // Tab picks a box; it changes no tile.
        clock.sent(1, "key");
        clock.landed(1, Ok(None));
        clock.sent(2, "text");
        clock.landed(2, Ok(Some(10)));
        // Another deck's import repaints this slide meanwhile.
        clock.drawn(9);
        clock.sent(3, "text");
        clock.landed(3, Ok(Some(21)));
        assert_eq!(clock.pending(), [2, 3]);
        clock.drawn(19);
        assert_eq!(clock.pending(), [3], "each key has its own screen");
        clock.drawn(21);
        assert!(clock.pending().is_empty());
    }

    #[test]
    fn a_hidden_view_never_draws_its_edits() {
        let mut clock = shown();
        clock.sent(1, "text");
        clock.landed(1, Ok(Some(3)));
        clock.sent(2, "text");
        clock.hidden();
        assert!(clock.pending().is_empty(), "left behind, not timed later");
        clock.landed(2, Ok(Some(4)));
        clock.sent(3, "text");
        clock.showing();
        clock.drawn(9);
        assert!(!clock.waiting(), "edits made while hidden are not timed");
    }

    #[test]
    fn failed_and_unread_edits_never_draw() {
        let mut clock = shown();
        clock.sent(1, "text");
        clock.landed(1, picture(Some((1, 1)), Some((2, 2)), true));
        clock.sent(2, "text");
        clock.landed(2, picture(None, Some((2, 2)), false));
        assert!(!clock.waiting());
    }
}
