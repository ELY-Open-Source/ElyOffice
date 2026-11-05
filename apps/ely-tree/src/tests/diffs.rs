//! Diffs from Git, cut into hunk and line patches (C13.2).

use std::{collections::BTreeSet, path::Path, sync::Arc};

use ely_gpui_component::git::{ChangeSection, LineKind, Stretch};

use super::{committed, git, sh};
use crate::{
    diff::{self, Body, Space},
    shown::{self, Found},
    status::{self, Entry},
    work::{self, Write},
};

fn entry(root: &Path, path: &str) -> Entry {
    let read = status::read(&git(), root).expect("reads");
    read.entries
        .into_iter()
        .find(|entry| entry.path == path)
        .expect("listed")
}

fn read(
    root: &Path,
    path: &str,
    section: ChangeSection,
) -> (diff::FileDiff, Vec<u8>, Option<String>) {
    let born = status::read(&git(), root)
        .expect("reads")
        .head
        .oid
        .is_some();
    match shown::read(
        &git(),
        root,
        &entry(root, path),
        section,
        Space::Exact,
        born,
    )
    .expect("reads")
    {
        Found::Text { diff, seen, old } => (diff, seen, old),
        _ => panic!("a text diff"),
    }
}

/// Applies the chosen `(hunk, line)`s as the app does.
fn apply(
    root: &Path,
    path: &str,
    section: ChangeSection,
    chosen: &[(usize, usize)],
) -> std::io::Result<()> {
    let (diff, seen, _) = read(root, path, section);
    let reverse = section == ChangeSection::Staged;
    let chosen: BTreeSet<_> = chosen.iter().copied().collect();
    let write = Write::Patch {
        patch: diff::patch(&diff, &chosen, reverse).expect("a patch"),
        reverse,
        check: shown::args(&entry(root, path), section, Space::Exact),
        seen: Arc::new(seen),
    };
    work::run(&git(), root, &write, &write.args(true))
}

fn index(root: &Path, path: &str) -> String {
    sh(root, &["cat-file", "blob", &format!(":{path}")])
}

fn numbered(lines: std::ops::RangeInclusive<usize>) -> String {
    lines.map(|n| format!("{n}\n")).collect()
}

#[test]
fn a_diff_reads_hunks_missing_newlines_and_binaries() {
    let out = b"diff --git a/a b/a\nindex 1..2 100644\n--- a/a\n+++ b/a\n@@ -1,2 +1,2 @@ fn x\n a\n-b\n\\ No newline at end of file\n+c\n\\ No newline at end of file\n";
    let read = diff::parse(out).expect("reads");
    let Body::Text(hunks) = &read.body else {
        panic!("text")
    };
    assert_eq!(
        (hunks[0].old, hunks[0].new, hunks[0].tail.as_str()),
        ((1, 2), (1, 2), "fn x")
    );
    let shape: Vec<_> = hunks[0]
        .lines
        .iter()
        .map(|raw| (raw.kind, raw.text.as_str(), raw.newline))
        .collect();
    assert_eq!(
        shape,
        [
            (LineKind::Same, "a", true),
            (LineKind::Removed, "b", false),
            (LineKind::Added, "c", false)
        ]
    );
    assert!(read.partial());
    let binary = diff::parse(b"diff --git a/p b/p\nindex 1..2\nBinary files a/p and b/p differ\n")
        .expect("reads");
    assert_eq!(binary.body, Body::Binary);
    assert_eq!(
        diff::parse(b"\xff\xfe").expect("reads").body,
        Body::Undecodable
    );
    assert!(diff::parse(b"@@ -1 +1 @@\n?what\n").is_err());
}

#[test]
fn one_hunk_stages_and_the_other_stays() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("a.txt"), numbered(1..=20)).expect("writes");
    sh(&root, &["commit", "-qam", "twenty"]);
    let word = |n: usize| match n {
        2 => "two\n".to_string(),
        18 => "eighteen\n".to_string(),
        n => format!("{n}\n"),
    };
    let changed: String = (1..=20).map(word).collect();
    std::fs::write(root.join("a.txt"), &changed).expect("writes");
    let (diff, _, old) = read(&root, "a.txt", ChangeSection::Unstaged);
    let (stretches, hunks) = shown::stretches(&diff, old.as_deref()).expect("stretches");
    assert_eq!(hunks, [Some(0), None, Some(1)], "two hunks, a fold between");
    let Stretch::Folded(between) = &stretches[1] else {
        panic!("a fold")
    };
    assert_eq!((between[0].old, between[0].new), (Some(6), Some(6)));
    // The second hunk's lines: three context, removed, added, ...
    apply(&root, "a.txt", ChangeSection::Unstaged, &[(1, 3), (1, 4)]).expect("stages");
    let only: String = (1..=20)
        .map(|n| {
            if n == 18 {
                "eighteen\n".into()
            } else {
                format!("{n}\n")
            }
        })
        .collect();
    assert_eq!(index(&root, "a.txt"), only);
    let (rest, _, _) = read(&root, "a.txt", ChangeSection::Unstaged);
    let Body::Text(rest) = rest.body else {
        panic!("text")
    };
    assert_eq!(rest.len(), 1, "the first hunk is still unstaged");
}

#[test]
fn single_lines_stage_and_unstage_as_picked() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("a.txt"), "one\nthree\nfour\n").expect("writes");
    // Hunk: one, -two, +three, +four.
    apply(&root, "a.txt", ChangeSection::Unstaged, &[(0, 3)]).expect("stages one added line");
    assert_eq!(
        index(&root, "a.txt"),
        "one\ntwo\nfour\n",
        "only four, two kept"
    );
    apply(&root, "a.txt", ChangeSection::Unstaged, &[(0, 1)]).expect("stages one removal");
    assert_eq!(index(&root, "a.txt"), "one\nfour\n");
    sh(&root, &["add", "a.txt"]);
    let (staged, _, _) = read(&root, "a.txt", ChangeSection::Staged);
    let Body::Text(hunks) = &staged.body else {
        panic!("text")
    };
    let three = hunks[0]
        .lines
        .iter()
        .position(|raw| raw.text == "three")
        .expect("three");
    apply(&root, "a.txt", ChangeSection::Staged, &[(0, three)]).expect("unstages one line");
    assert_eq!(index(&root, "a.txt"), "one\nfour\n", "three goes back out");
    assert_eq!(
        std::fs::read_to_string(root.join("a.txt")).expect("reads"),
        "one\nthree\nfour\n"
    );
}

#[test]
fn a_last_line_without_newline_stages() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("a.txt"), "one\nend").expect("writes");
    sh(&root, &["commit", "-qam", "no newline"]);
    std::fs::write(root.join("a.txt"), "one\nfin").expect("writes");
    apply(&root, "a.txt", ChangeSection::Unstaged, &[(0, 1), (0, 2)]).expect("stages");
    assert_eq!(index(&root, "a.txt"), "one\nfin");
}

#[test]
fn a_patch_from_an_older_diff_is_refused() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("a.txt"), "one\nTWO\n").expect("writes");
    let (diff, seen, _) = read(&root, "a.txt", ChangeSection::Unstaged);
    std::fs::write(root.join("a.txt"), "ONE\nTWO\n").expect("writes");
    let chosen: BTreeSet<_> = [(0, 1), (0, 2)].into();
    let write = Write::Patch {
        patch: diff::patch(&diff, &chosen, false).expect("a patch"),
        reverse: false,
        check: shown::args(
            &entry(&root, "a.txt"),
            ChangeSection::Unstaged,
            Space::Exact,
        ),
        seen: Arc::new(seen),
    };
    let refused = work::run(&git(), &root, &write, &write.args(true)).expect_err("stale");
    assert_eq!(refused.to_string(), "tree.error.patch_stale");
    assert_eq!(
        index(&root, "a.txt"),
        "one\ntwo\n",
        "the index is untouched"
    );
}

#[test]
fn whole_files_pictures_and_binaries_read_as_such() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("new.txt"), "fresh\n").expect("writes");
    let (untracked, _, old) = read(&root, "new.txt", ChangeSection::Untracked);
    assert!(
        !untracked.partial() && old.is_none(),
        "a new file stages whole"
    );
    sh(&root, &["mv", "a.txt", "b.txt"]);
    let (renamed, _, _) = read(&root, "b.txt", ChangeSection::Staged);
    assert!(
        renamed
            .head
            .iter()
            .any(|line| line.starts_with("rename from a.txt"))
            && !renamed.partial()
    );
    let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    std::fs::write(root.join("p.png"), &png).expect("writes");
    std::fs::write(root.join("blob.bin"), b"\0\x01\x02").expect("writes");
    let born = true;
    let picture = shown::read(
        &git(),
        &root,
        &entry(&root, "p.png"),
        ChangeSection::Untracked,
        Space::Exact,
        born,
    )
    .expect("reads");
    assert!(matches!(picture, Found::Picture { old: None, new: Some(bytes), .. } if bytes == png));
    let binary = shown::read(
        &git(),
        &root,
        &entry(&root, "blob.bin"),
        ChangeSection::Untracked,
        Space::Exact,
        born,
    )
    .expect("reads");
    assert!(matches!(
        binary,
        Found::Binary {
            old: None,
            new: Some(3)
        }
    ));
}

#[test]
fn ignoring_whitespace_hides_a_space_only_change() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("a.txt"), "one  \ntwo\n").expect("writes");
    let entry = entry(&root, "a.txt");
    let found = shown::read(
        &git(),
        &root,
        &entry,
        ChangeSection::Unstaged,
        Space::All,
        true,
    )
    .expect("reads");
    let Found::Text { diff, .. } = found else {
        panic!("text")
    };
    assert_eq!(diff.body, Body::Text(Vec::new()));
}

#[test]
fn a_kept_last_line_gains_its_newline_when_a_line_follows() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("a.txt"), "one\ntwo").expect("writes");
    sh(&root, &["commit", "-qam", "no newline"]);
    std::fs::write(root.join("a.txt"), "one\nTWO\nmore\n").expect("writes");
    // one, -two, +TWO, +more: only +more.
    apply(&root, "a.txt", ChangeSection::Unstaged, &[(0, 3)]).expect("stages");
    assert_eq!(index(&root, "a.txt"), "one\ntwo\nmore\n");
    sh(&root, &["add", "a.txt"]);
    let (staged, _, _) = read(&root, "a.txt", ChangeSection::Staged);
    let Body::Text(hunks) = &staged.body else {
        panic!("text")
    };
    let more = hunks[0]
        .lines
        .iter()
        .position(|raw| raw.text == "more")
        .expect("more");
    apply(&root, "a.txt", ChangeSection::Staged, &[(0, more)]).expect("unstages");
    assert_eq!(index(&root, "a.txt"), "one\nTWO\n");
}

#[test]
fn picked_bytes_go_in_whatever_apply_whitespace_says() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    sh(&root, &["config", "apply.whitespace", "fix"]);
    std::fs::write(root.join("a.txt"), "one\nTWO  \n").expect("writes");
    apply(&root, "a.txt", ChangeSection::Unstaged, &[(0, 1), (0, 2)]).expect("stages");
    assert_eq!(index(&root, "a.txt"), "one\nTWO  \n");
}

#[test]
fn bytes_that_are_not_utf8_outside_the_diff_show_by_size() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    let mut bytes = b"\xff\n".to_vec();
    bytes.extend(numbered(2..=30).into_bytes());
    std::fs::write(root.join("a.txt"), &bytes).expect("writes");
    sh(&root, &["commit", "-qam", "latin"]);
    bytes.extend(b"last\n");
    std::fs::write(root.join("a.txt"), &bytes).expect("writes");
    let found = shown::read(
        &git(),
        &root,
        &entry(&root, "a.txt"),
        ChangeSection::Unstaged,
        Space::Exact,
        true,
    )
    .expect("reads");
    assert!(matches!(found, Found::Binary { .. }), "not a text view");
}

#[test]
fn blank_context_lines_read_whatever_the_config() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("a.txt"), "one\n\ntwo\n").expect("writes");
    sh(&root, &["commit", "-qam", "blank"]);
    sh(&root, &["config", "diff.suppressBlankEmpty", "true"]);
    std::fs::write(root.join("a.txt"), "one\n\nTWO\n").expect("writes");
    let (diff, _, _) = read(&root, "a.txt", ChangeSection::Unstaged);
    assert!(diff.partial());
    apply(&root, "a.txt", ChangeSection::Unstaged, &[(0, 2), (0, 3)]).expect("stages");
    assert_eq!(index(&root, "a.txt"), "one\n\nTWO\n");
}

#[test]
fn an_empty_old_file_folds_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("a.txt"), "").expect("writes");
    sh(&root, &["commit", "-qam", "empty"]);
    std::fs::write(root.join("a.txt"), "hello\n").expect("writes");
    let (diff, _, old) = read(&root, "a.txt", ChangeSection::Unstaged);
    let (stretches, hunks) = shown::stretches(&diff, old.as_deref()).expect("stretches");
    assert_eq!(hunks, [Some(0)], "one hunk, no fold");
    let Stretch::Hunk { lines, .. } = &stretches[0] else {
        panic!("a hunk")
    };
    assert_eq!(lines.len(), 1);
}

#[test]
fn a_restored_last_line_keeps_its_newline_when_lines_follow() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("a.txt"), "one\ntwo").expect("writes");
    sh(&root, &["commit", "-qam", "no newline"]);
    std::fs::write(root.join("a.txt"), "one\nTWO\nmore\n").expect("writes");
    sh(&root, &["add", "a.txt"]);
    let (staged, _, _) = read(&root, "a.txt", ChangeSection::Staged);
    let Body::Text(hunks) = &staged.body else {
        panic!("text")
    };
    let two = hunks[0]
        .lines
        .iter()
        .position(|raw| raw.text == "two")
        .expect("two");
    apply(&root, "a.txt", ChangeSection::Staged, &[(0, two)]).expect("unstages");
    assert_eq!(index(&root, "a.txt"), "one\ntwo\nTWO\nmore\n");
}

#[cfg(unix)]
#[test]
fn a_file_turned_into_a_link_reads_in_both_sections() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("other.txt"), "other\n").expect("writes");
    std::fs::remove_file(root.join("a.txt")).expect("removes");
    std::os::unix::fs::symlink("other.txt", root.join("a.txt")).expect("links");
    for section in [ChangeSection::Unstaged, ChangeSection::Staged] {
        if section == ChangeSection::Staged {
            sh(&root, &["add", "a.txt"]);
        }
        let (diff, _, old) = read(&root, "a.txt", section);
        assert!(!diff.partial(), "{section:?} stages whole");
        let (stretches, _) = shown::stretches(&diff, old.as_deref()).expect("stretches");
        // Every row, folds included, with its numbers.
        let rows: Vec<(String, Option<usize>, Option<usize>)> = stretches
            .iter()
            .flat_map(|stretch| match stretch {
                Stretch::Hunk { lines, .. } | Stretch::Folded(lines) => lines.iter(),
            })
            .map(|line| (line.text.to_string(), line.old, line.new))
            .collect();
        let expected = [
            ("one".to_string(), Some(1), None),
            ("two".to_string(), Some(2), None),
            ("other.txt".to_string(), None, Some(1)),
        ];
        assert_eq!(rows, expected, "{section:?}: the text, then the link");
    }
}

#[cfg(unix)]
#[test]
fn a_link_compares_by_its_own_path_whatever_it_points_at() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::remove_file(root.join("a.txt")).expect("removes");
    std::os::unix::fs::symlink("missing.txt", root.join("a.txt")).expect("links");
    let (diff, _, _) = read(&root, "a.txt", ChangeSection::Unstaged);
    assert!(
        format!("{diff:?}").contains("missing.txt"),
        "a dangling link reads"
    );
    std::fs::write(root.join("latin.txt"), b"\xff\n").expect("writes");
    std::fs::remove_file(root.join("a.txt")).expect("removes");
    std::os::unix::fs::symlink("latin.txt", root.join("a.txt")).expect("links");
    let (diff, _, _) = read(&root, "a.txt", ChangeSection::Unstaged);
    assert!(
        format!("{diff:?}").contains("latin.txt"),
        "a link to bytes is text"
    );
}

#[cfg(unix)]
#[test]
fn a_picture_turned_into_a_link_shows_the_link_by_size() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    std::fs::write(root.join("p.png"), &png).expect("writes");
    sh(&root, &["add", "p.png"]);
    sh(&root, &["commit", "-qm", "picture"]);
    std::fs::remove_file(root.join("p.png")).expect("removes");
    std::os::unix::fs::symlink("q.png", root.join("p.png")).expect("links");
    for section in [ChangeSection::Unstaged, ChangeSection::Staged] {
        if section == ChangeSection::Staged {
            sh(&root, &["add", "p.png"]);
        }
        let found = shown::read(&git(), &root, &entry(&root, "p.png"), section, Space::Exact, true)
            .expect("reads");
        let want = (Some(png.len() as u64), Some("q.png".len() as u64));
        assert!(
            matches!(found, Found::Binary { old, new } if (old, new) == want),
            "{section:?}: the link is no picture"
        );
    }
    let seen = sh(&root, &["diff", "--cached", "--", "p.png"]);
    let diff = diff::parse(seen.as_bytes()).expect("parses");
    assert_eq!(diff.sections(), 2, "both sections read: {seen}");
    assert_eq!(diff.body, Body::Binary);
}
