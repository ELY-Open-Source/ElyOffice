//! One tab's points: what was written, what failed.

use std::{io, time::SystemTime};

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
