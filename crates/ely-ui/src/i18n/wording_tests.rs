use super::catalogs;
use std::collections::{BTreeMap, BTreeSet};

/// Each English label's renderings in one catalog.
fn renderings(english: &[(&str, &str)], other: &[(&str, &str)], labels: &[&str]) -> BTreeMap<String, BTreeSet<String>> {
    let other: BTreeMap<_, _> = other.iter().copied().collect();
    let mut found: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (key, label) in english.iter().filter(|(_, label)| labels.contains(label)) {
        found.entry(label.to_string()).or_default().insert(other[key].to_string());
    }
    found
}

/// Commands that write, drop or send, as English names them.
const APART: [&str; 13] = [
    "Save", "Save As…", "Save a Copy…", "Export…", "Don't Save", "Discard", "Delete", "Undo", "Redo", "Send",
    "Keep Mine", "Reload from Disk", "Rename",
];

#[test]
fn commands_that_change_data_keep_their_own_words() {
    let [english, rest @ ..] = catalogs();
    for (index, catalog) in rest.iter().enumerate() {
        let found = renderings(&english, catalog, &APART);
        assert_eq!(found.len(), APART.len(), "every label is in use");
        let mut owner: BTreeMap<&String, &String> = BTreeMap::new();
        for (label, words) in &found {
            for word in words {
                if let Some(other) = owner.insert(word, label) {
                    panic!("catalog {index}: {word} stands for both {other} and {label}");
                }
            }
        }
    }
}

/// Commands every app shares: one word each, per language.
const ALIKE: [&str; 12] =
    ["Save", "Save As…", "Export…", "Delete", "Undo", "Redo", "Cut", "Copy", "Paste", "Rename", "Cancel", "Print…"];

#[test]
fn a_shared_command_reads_the_same_in_every_app() {
    let [english, rest @ ..] = catalogs();
    for (index, catalog) in rest.iter().enumerate() {
        let found = renderings(&english, catalog, &ALIKE);
        assert_eq!(found.len(), ALIKE.len(), "every label is in use");
        for (label, words) in found {
            assert_eq!(words.len(), 1, "catalog {index}: {label} reads as {words:?}");
        }
    }
}
