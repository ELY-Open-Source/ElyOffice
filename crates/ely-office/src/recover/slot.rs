//! One tab's points: what was written, what failed.

use std::{io, time::{Instant, SystemTime}};

use super::Kept;

/// A tab's points, and the edit the last holds.
pub(super) struct Slot {
    pub kept: Kept,
    pub written: Option<u64>,
    pub writing: bool,
    /// The last failure, shown once until it changes.
    pub failed: Option<String>,
    /// Bytes of a refused point, or of a later save.
    pub refused: Option<u64>,
    /// When the last point was written.
    pub last: Option<SystemTime>,
    /// When input first stopped a point not yet written.
    pub stopped: Option<Instant>,
}

impl Slot {
    /// Notes a failed point; words to show, once each.
    pub fn fail(&mut self, size: Option<u64>, error: &io::Error) -> Option<String> {
        // Too big: no points until it fits again.
        if error.kind() == io::ErrorKind::FileTooLarge {
            self.refused = size;
        }
        let said = error.to_string();
        (self.failed.as_ref() != Some(&said)).then(|| {
            self.failed = Some(said.clone());
            said
        })
    }
}

impl super::Keeping {
    /// Tab `id`'s recovery point is being written.
    pub(crate) fn writes(&self, id: u64) -> bool {
        self.slots.get(&id).is_some_and(|slot| slot.writing)
    }
}

#[cfg(test)]
impl super::Keeping {
    /// Marks tab `id`'s point as being written.
    pub(crate) fn set_writing(&mut self, id: u64, writing: bool) {
        self.slots.get_mut(&id).expect("a kept tab").writing = writing;
    }
}
