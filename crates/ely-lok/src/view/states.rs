//! Kernel states, and whether the modified one is current.

use std::collections::HashMap;

use super::LokView;

const MODIFIED: &str = ".uno:ModifiedStatus";

/// The states as last reported, and when.
#[derive(Default)]
pub(super) struct States {
    named: HashMap<String, String>,
    /// The revision of the last modified report.
    modified_at: u64,
    /// The revision after the last landed edit.
    edited_at: u64,
}

impl States {
    pub(super) fn get(&self, uno: &str) -> Option<&str> {
        self.named.get(uno).map(String::as_str)
    }

    /// A batch at `revision` changed these states.
    pub(super) fn take(&mut self, changed: Vec<(String, String)>, revision: u64) {
        if changed.iter().any(|(name, _)| name == MODIFIED) {
            self.modified_at = revision;
        }
        self.named.extend(changed);
    }

    /// An edit landed; `now` is the revision after it.
    pub(super) fn edited(&mut self, now: Option<u64>, seen: u64) {
        self.edited_at = now.unwrap_or(seen + 1);
    }

    fn unconfirmed(&self) -> bool {
        self.get(MODIFIED) != Some("true") && self.edited_at > self.modified_at
    }
}

impl LokView {
    /// An edit is out or landed, and no report answered.
    pub fn unconfirmed(&self) -> bool {
        self.states.unconfirmed()
    }
}

#[cfg(test)]
mod tests {
    use super::{MODIFIED, States};

    fn modified(states: &mut States, value: &str, revision: u64) {
        states.take(vec![(MODIFIED.to_string(), value.to_string())], revision);
    }

    #[test]
    fn an_edit_stays_unsaved_until_a_later_report_answers_it() {
        let mut states = States::default();
        assert!(!states.unconfirmed(), "nothing edited");
        states.edited(Some(5), 4);
        assert!(states.unconfirmed(), "no report yet");
        modified(&mut states, "false", 4);
        assert!(states.unconfirmed(), "a report before the edit says nothing");
        modified(&mut states, "true", 6);
        assert!(!states.unconfirmed(), "modified is dirty on its own");
        states.edited(Some(7), 6);
        modified(&mut states, "false", 8);
        assert!(!states.unconfirmed(), "an undo back to the file is clean");
        states.take(vec![(".uno:Bold".into(), "true".into())], 9);
        states.edited(None, 9);
        assert!(states.unconfirmed(), "an edit with no revision is newer than all seen");
        modified(&mut states, "false", 10);
        states.edited(Some(11), 10);
        assert!(states.unconfirmed(), "a landed write waits for its report");
        modified(&mut states, "true", 11);
        assert!(!states.unconfirmed(), "the report answered it");
    }
}
