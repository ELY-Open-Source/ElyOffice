use super::*;

fn entry(path: &str, from: Option<&str>, state: State) -> Entry {
    Entry {
        path: path.into(),
        from: from.map(Into::into),
        state,
    }
}

#[test]
fn each_write_names_its_paths_after_the_separator() {
    let renamed = entry(
        "new.txt",
        Some("old.txt"),
        State::Tracked {
            staged: 'R',
            unstaged: '.',
        },
    );
    assert_eq!(
        Write::Unstage(renamed.clone()).args(true),
        ["restore", "--staged", "--", "new.txt", "old.txt"]
    );
    assert_eq!(
        Write::Discard(renamed).args(true),
        ["restore", "--worktree", "--", "new.txt"],
        "the old path is gone"
    );
    let untracked = entry("-n", None, State::Untracked);
    assert_eq!(
        Write::Discard(untracked).args(true),
        ["clean", "-f", "-d", "-q", "--", "-n"]
    );
    let changed = entry(
        "a.txt",
        None,
        State::Tracked {
            staged: '.',
            unstaged: 'M',
        },
    );
    assert_eq!(
        Write::Discard(changed.clone()).args(true),
        ["restore", "--worktree", "--", "a.txt"]
    );
    assert_eq!(
        Write::Unstage(changed).args(false),
        ["rm", "--cached", "-f", "-r", "-q", "--", "a.txt"]
    );
    assert_eq!(Write::UnstageAll.args(true), ["reset", "-q"]);
}
