//! Branches, operations and conflicts, in the window (C13.4).

use ely_gpui_component::git::{ChangeSection, Take};
use gpui::{App, TestAppContext};

use super::{
    branching::conflicted,
    sh,
    window::{branch, settle, window},
};
use crate::{asks::Ask, branches::Kind, tree::Side};

fn conflicts(tree: &crate::tree::Tree) -> usize {
    match tree.active().and_then(|opened| opened.status.as_ref()) {
        Some(Ok(status)) => status
            .entries
            .iter()
            .filter(|e| matches!(e.state, crate::status::State::Conflicted(_)))
            .count(),
        _ => 0,
    }
}

#[test]
fn a_merge_conflict_is_settled_saved_resolved_and_continued() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (root, _) = conflicted(dir.path());
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.merge("side", cx)));
    settle(cx, |app| conflicts(tree.read(app)) == 1);
    let doing = |app: &App| {
        matches!(
            tree.read(app).active().and_then(|o| o.doing.clone()),
            Some(Ok(Some(_)))
        )
    };
    settle(cx, doing);
    assert!(
        cx.update(|_, app| tree.read(app).ask.is_none()),
        "a stop is not a failure dialog"
    );
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.show("a.txt", ChangeSection::Conflicted, cx)
        })
    });
    settle(cx, |app| tree.read(app).merging.is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.take(0, Take::Theirs, cx)));
    let result = cx.update(|_, app| tree.read(app).merge_editor.read(app).text().to_string());
    assert_eq!(result, "one\nside\n");
    // Resolving before saving is refused; saving writes the file.
    cx.update(|_, app| tree.update(app, |tree, cx| tree.mark_resolved(cx)));
    assert_eq!(
        sh(&root, &["diff", "--name-only", "--diff-filter=U"]).trim(),
        "a.txt"
    );
    cx.update(|_, app| tree.update(app, |tree, cx| tree.save_merge(cx)));
    settle(cx, |_| {
        std::fs::read_to_string(root.join("a.txt")).expect("reads") == "one\nside\n"
    });
    settle(cx, |app| {
        tree.read(app)
            .merging
            .as_ref()
            .is_some_and(|m| m.saved == "one\nside\n")
    });
    cx.update(|_, app| tree.update(app, |tree, cx| tree.mark_resolved(cx)));
    settle(cx, |app| conflicts(tree.read(app)) == 0);
    cx.update(|_, app| tree.update(app, |tree, cx| tree.op_continue(crate::ops::Op::Merge, cx)));
    settle(cx, |_| {
        sh(&root, &["log", "-1", "--format=%p"])
            .split_whitespace()
            .count()
            == 2
    });
    settle(cx, |app| {
        matches!(
            tree.read(app).active().and_then(|o| o.doing.clone()),
            Some(Ok(None))
        )
    });
}

#[test]
fn branches_are_made_renamed_and_deleted_each_with_its_own_yes() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (root, main) = conflicted(dir.path());
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.set_side(Side::Branches, cx)));
    let names = |app: &App| match tree.read(app).active().and_then(|o| o.refs.clone()) {
        Some(Ok(refs)) => refs
            .into_iter()
            .filter(|r| r.kind == Kind::Local)
            .map(|r| r.name)
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    settle(cx, |app| names(app).len() == 2);
    cx.update(|window, app| tree.update(app, |tree, cx| tree.ask_branch(None, window, cx)));
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            if let Some(Ask::NewBranch { name, .. }) = &tree.ask {
                name.update(cx, |field, cx| field.set_text("topic".to_string(), cx));
            }
            tree.answer(cx);
        })
    });
    settle(cx, |app| branch(tree.read(app)).as_deref() == Some("topic"));
    cx.update(|window, app| tree.update(app, |tree, cx| tree.ask_rename("topic", window, cx)));
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            if let Some(Ask::Rename { name, .. }) = &tree.ask {
                name.update(cx, |field, cx| field.set_text("renamed".to_string(), cx));
            }
            tree.answer(cx);
        })
    });
    settle(cx, |app| {
        branch(tree.read(app)).as_deref() == Some("renamed")
    });
    sh(&root, &["checkout", "-q", &main]);
    settle(cx, |app| {
        branch(tree.read(app)).as_deref() == Some(main.as_str())
    });
    // Delete waits for its own yes; nothing runs before it.
    cx.update(|_, app| tree.update(app, |tree, cx| tree.delete_branch("renamed", false, cx)));
    assert!(matches!(
        cx.update(|_, app| tree
            .read(app)
            .ask
            .as_ref()
            .map(|a| matches!(a, Ask::Confirm { .. }))),
        Some(true)
    ));
    assert!(names_now(&root).contains(&"renamed".to_string()));
    let write = cx.update(|_, app| match &tree.read(app).ask {
        Some(Ask::Confirm { write, .. }) => write.clone(),
        _ => panic!("a confirm"),
    });
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.ask = None;
            tree.write(1, write, cx)
        })
    });
    settle(cx, |_| !names_now(&root).contains(&"renamed".to_string()));
    // The next delete asks again.
    cx.update(|_, app| tree.update(app, |tree, cx| tree.delete_branch("side", false, cx)));
    assert!(cx.update(|_, app| tree.read(app).ask.is_some()));
}

fn names_now(root: &std::path::Path) -> Vec<String> {
    sh(root, &["branch", "--format=%(refname:short)"])
        .lines()
        .map(String::from)
        .collect()
}

#[test]
fn a_failure_that_is_no_stop_shows_gits_words() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (root, _) = conflicted(dir.path());
    std::fs::write(root.join("a.txt"), "dirty\n").expect("writes");
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    // A dirty file blocks the switch; Git says so.
    cx.update(|_, app| tree.update(app, |tree, cx| tree.switch("side", Kind::Local, cx)));
    settle(
        cx,
        |app| matches!(&tree.read(app).ask, Some(Ask::Failed { detail, .. }) if detail.contains("a.txt")),
    );
}

#[cfg(unix)]
#[test]
fn a_hook_that_stops_a_continue_says_why() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::tempdir().expect("a temp dir");
    let (root, _) = conflicted(dir.path());
    assert!(super::git().run(&root, &["merge", "side"]).is_err());
    std::fs::write(root.join("a.txt"), "one\nsettled\n").expect("writes");
    sh(&root, &["add", "a.txt"]);
    let hook = root.join(".git/hooks/pre-commit");
    std::fs::write(
        &hook,
        "#!/bin/sh\necho 'CONFLICT (hook): REVIEW_HOOK_DENIED'\nexit 1\n",
    )
    .expect("writes");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.op_continue(crate::ops::Op::Merge, cx)));
    settle(
        cx,
        |app| matches!(&tree.read(app).ask, Some(Ask::Failed { detail, .. }) if detail.contains("REVIEW_HOOK_DENIED")),
    );
}

#[cfg(unix)]
#[test]
fn a_hook_that_stops_a_side_take_says_why_while_conflicts_remain() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::tempdir().expect("a temp dir");
    let (root, _) = conflicted(dir.path());
    assert!(super::git().run(&root, &["merge", "side"]).is_err());
    let hook = root.join(".git/hooks/post-checkout");
    std::fs::write(
        &hook,
        "#!/bin/sh\necho REVIEW_POST_CHECKOUT_DENIED >&2\nexit 1\n",
    )
    .expect("writes");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| conflicts(tree.read(app)) == 1);
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.show("a.txt", ChangeSection::Conflicted, cx)
        })
    });
    settle(cx, |app| tree.read(app).merging.is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.take_side(true, cx)));
    settle(
        cx,
        |app| matches!(&tree.read(app).ask, Some(Ask::Failed { detail, .. }) if detail.contains("REVIEW_POST_CHECKOUT_DENIED")),
    );
}

#[test]
fn a_pick_that_conflicts_stops_without_a_dialog() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (root, _) = conflicted(dir.path());
    let theirs = sh(&root, &["rev-parse", "side"]).trim().to_string();
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.cherry_pick(&theirs, 1, cx)));
    settle(cx, |app| conflicts(tree.read(app)) == 1);
    settle(cx, |app| {
        matches!(
            tree.read(app).active().and_then(|o| o.doing.clone()),
            Some(Ok(Some(_)))
        )
    });
    assert!(cx.update(|_, app| tree.read(app).ask.is_none()));
}

#[test]
fn a_git_am_stopped_on_a_conflict_says_so() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (root, _) = conflicted(dir.path());
    let patch = sh(&root, &["format-patch", "-1", "side", "--stdout"]);
    std::fs::write(dir.path().join("p.patch"), patch).expect("writes");
    let patch_at = dir.path().join("p.patch").to_str().expect("utf-8").to_owned();
    // `git am` stops on the conflict, as in a terminal.
    let stopped = super::git().run(&root, &["am", "--3way", patch_at.as_str()]);
    assert!(stopped.is_err(), "the patch conflicts");
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    let op = |app: &App| match tree.read(app).active().and_then(|o| o.doing.clone()) {
        Some(Ok(Some(doing))) => Some((doing.op, doing.theirs)),
        _ => None,
    };
    settle(cx, |app| op(app).is_some());
    assert_eq!(
        cx.update(|_, app| op(app)),
        Some((crate::ops::Op::Am, "1/1".to_string()))
    );
    let applying = || root.join(".git/rebase-apply").exists();
    // Skip, then abort a second stop: each asks first.
    for skip in [true, false] {
        if !skip {
            assert!(super::git().run(&root, &["am", "--3way", patch_at.as_str()]).is_err());
            settle(cx, |app| op(app).is_some());
        }
        cx.update(|_, app| {
            tree.update(app, |tree, cx| match skip {
                true => tree.op_skip(crate::ops::Op::Am, cx),
                false => tree.op_abort(crate::ops::Op::Am, cx),
            })
        });
        let write = cx.update(|_, app| match &tree.read(app).ask {
            Some(Ask::Confirm { write, .. }) => write.clone(),
            _ => panic!("a confirm"),
        });
        assert!(applying(), "nothing runs before the yes");
        cx.update(|_, app| {
            tree.update(app, |tree, cx| {
                tree.ask = None;
                tree.write(1, write, cx)
            })
        });
        settle(cx, |_| !applying());
    }
}

#[test]
fn a_settled_conflict_says_so_when_picked_again() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (root, _) = conflicted(dir.path());
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    let shell = cx.update(|window, app| {
        let shell = ely_ui::tool_root(
            ely_platform::AppId::Tree,
            tree.clone().into(),
            Vec::new(),
            window,
            app,
        );
        tree.update(app, |tree, _| tree.root = shell.downgrade());
        shell
    });
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.merge("side", cx)));
    settle(cx, |app| conflicts(tree.read(app)) == 1);
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.show("a.txt", ChangeSection::Conflicted, cx)
        })
    });
    settle(cx, |app| tree.read(app).merging.is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.take(0, Take::Theirs, cx)));
    cx.update(|_, app| tree.update(app, |tree, cx| tree.take(0, Take::Ours, cx)));
    let want = cx.update(|_, app| ely_ui::i18n::text(app, "tree.conflict.taken", &[]));
    let toasts = cx.update(|_, app| shell.read(app).toasts(app));
    assert!(toasts.iter().any(|(text, _)| *text == want), "{toasts:?}");
}

#[test]
fn a_stop_that_does_not_read_says_why() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = super::committed(dir.path());
    // A merge whose other side no longer exists.
    let gone = "0123456789abcdef0123456789abcdef01234567";
    std::fs::write(root.join(".git/MERGE_HEAD"), format!("{gone}\n")).expect("writes");
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    let unread = |app: &App| match tree.read(app).active().and_then(|o| o.doing.clone()) {
        Some(Err(error)) => Some(error),
        _ => None,
    };
    settle(cx, |app| unread(app).is_some());
    let error = cx.update(|_, app| unread(app)).expect("an error");
    assert!(error.contains("bad object MERGE_HEAD"), "Git's own words: {error}");
    let alert = cx.update(|_, app| tree.update(app, |tree, cx| tree.op_alert(cx).is_some()));
    assert!(alert, "the failure shows above the work tree");
}

#[test]
fn a_git_am_skipped_onto_the_next_conflict_stops_without_a_dialog() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (root, main) = conflicted(dir.path());
    // Three patches, each on the line main changed.
    sh(&root, &["checkout", "-q", "side"]);
    for text in ["one\nside two\n", "one\nside three\n"] {
        std::fs::write(root.join("a.txt"), text).expect("writes");
        sh(&root, &["commit", "-qam", text]);
    }
    sh(&root, &["checkout", "-q", &main]);
    let patches = sh(&root, &["format-patch", &format!("{main}..side"), "--stdout"]);
    let at = dir.path().join("p.mbox");
    std::fs::write(&at, patches).expect("writes");
    let stopped = super::git().run(&root, &["am", "--3way", at.to_str().expect("utf-8")]);
    assert!(stopped.is_err(), "the first patch conflicts");
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    let theirs = |app: &App| match tree.read(app).active().and_then(|o| o.doing.clone()) {
        Some(Ok(Some(doing))) => Some(doing.theirs),
        _ => None,
    };
    settle(cx, |app| theirs(app).as_deref() == Some("1/3"));
    cx.update(|_, app| tree.update(app, |tree, cx| tree.op_skip(crate::ops::Op::Am, cx)));
    let write = cx.update(|_, app| match &tree.read(app).ask {
        Some(Ask::Confirm { write, .. }) => write.clone(),
        _ => panic!("a confirm"),
    });
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.ask = None;
            tree.write(1, write, cx)
        })
    });
    settle(cx, |app| theirs(app).as_deref() == Some("2/3") && conflicts(tree.read(app)) == 1);
    let failed = cx.update(|_, app| matches!(tree.read(app).ask, Some(Ask::Failed { .. })));
    assert!(!failed, "a stop on the next patch is no failure");
}
