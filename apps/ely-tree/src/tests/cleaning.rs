//! Cleaning untracked files: preview, then only what was shown (C13.6).

use gpui::TestAppContext;

use super::{
    committed, git,
    recovery::run,
    sh,
    window::{branch, settle, window},
};
use crate::{asks::Ask, recover, work::Write};

#[test]
fn a_clean_removes_what_was_shown_and_nothing_new() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join(".gitignore"), "*.log\n").expect("writes");
    sh(&root, &["add", ".gitignore"]);
    sh(&root, &["commit", "-qm", "ignore logs"]);
    // Windows names hold no tab or quote.
    let odd: &[&str] = match cfg!(windows) {
        true => &["中文.txt"],
        false => &["say \"hi\".txt", "tab\there.txt", "中文.txt"],
    };
    for name in odd.iter().chain(&["build.log"]) {
        std::fs::write(root.join(name), "x").expect("writes");
    }
    std::fs::create_dir_all(root.join("out/deep")).expect("makes");
    std::fs::create_dir_all(root.join("empty")).expect("makes");
    std::fs::write(root.join("out/deep/f.txt"), "x").expect("writes");
    std::fs::write(root.join("out/deep/kept.log"), "x").expect("writes");
    let clean = recover::clean_preview(&git(), &root, false).expect("previews");
    let mut files = clean.files.clone();
    files.sort();
    let shown: Vec<&str> = ["out/deep/f.txt"].iter().chain(odd).copied().collect();
    assert_eq!(files, shown);
    // `out/` holds an ignored file, so Git names its files.
    assert_eq!(clean.dirs, ["empty/"]);
    let with = recover::clean_preview(&git(), &root, true).expect("previews");
    assert!(with.files.contains(&"build.log".to_string()));
    assert!(with.files.contains(&"out/deep/kept.log".to_string()));
    // Late files, at the top and inside a shown folder.
    std::fs::write(root.join("late.txt"), "x").expect("writes");
    std::fs::write(root.join("out/late.txt"), "x").expect("writes");
    let write = Write::Clean {
        files: clean.files,
        dirs: clean.dirs,
        ignored: false,
    };
    run(&root, write).expect("cleans");
    assert!(root.join("late.txt").exists() && root.join("out/late.txt").exists());
    assert!(root.join("build.log").exists() && root.join("out/deep/kept.log").exists());
    assert!(!root.join("out/deep/f.txt").exists() && !root.join("empty").exists());
    assert!(odd.iter().all(|name| !root.join(name).exists()));
}

#[test]
fn the_clean_dialog_previews_then_cleans() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("junk.txt"), "x").expect("writes");
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.ask_clean(cx)));
    settle(
        cx,
        |app| matches!(&tree.read(app).ask, Some(Ask::Clean { preview: Some(Ok(clean)), .. }) if clean.files == ["junk.txt"]),
    );
    std::fs::write(root.join("late.txt"), "x").expect("writes");
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            let Some(Ask::Clean {
                repo,
                ignored,
                preview: Some(Ok(clean)),
            }) = tree.ask.take()
            else {
                panic!("a preview");
            };
            let write = Write::Clean {
                files: clean.files,
                dirs: clean.dirs,
                ignored,
            };
            tree.write(repo, write, cx);
        })
    });
    settle(cx, |_| !root.join("junk.txt").exists());
    assert!(root.join("late.txt").exists());
}


#[test]
fn a_shown_file_now_a_folder_stops_the_clean() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::create_dir_all(root.join("out")).expect("makes");
    std::fs::write(root.join("out/early.txt"), "x").expect("writes");
    std::fs::write(root.join("top.txt"), "x").expect("writes");
    let clean = recover::clean_preview(&git(), &root, false).expect("previews");
    std::fs::remove_file(root.join("out/early.txt")).expect("removes");
    std::fs::create_dir_all(root.join("out/early.txt")).expect("makes");
    std::fs::write(root.join("out/early.txt/late.txt"), "x").expect("writes");
    let write = Write::Clean {
        files: clean.files,
        dirs: clean.dirs,
        ignored: false,
    };
    let refused = run(&root, write).expect_err("a folder now");
    assert!(refused.to_string().contains("changed"), "{refused}");
    assert!(root.join("out/early.txt/late.txt").exists() && root.join("top.txt").exists());
}

#[test]
fn a_shown_file_staged_since_is_kept() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("new.txt"), "first\n").expect("writes");
    let clean = recover::clean_preview(&git(), &root, false).expect("previews");
    sh(&root, &["add", "new.txt"]);
    std::fs::write(root.join("new.txt"), "edited\n").expect("writes");
    let write = Write::Clean {
        files: clean.files,
        dirs: clean.dirs,
        ignored: false,
    };
    run(&root, write).expect("runs");
    assert_eq!(
        std::fs::read_to_string(root.join("new.txt")).expect("kept"),
        "edited\n"
    );
}

#[cfg(unix)]
#[test]
fn a_folder_turned_link_stops_the_clean() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::create_dir_all(root.join("out")).expect("makes");
    std::fs::write(root.join("out/early.txt"), "x").expect("writes");
    let clean = recover::clean_preview(&git(), &root, false).expect("previews");
    let outside = dir.path().join("outside");
    std::fs::create_dir_all(&outside).expect("makes");
    std::fs::write(outside.join("early.txt"), "mine").expect("writes");
    std::fs::rename(root.join("out"), root.join("saved-out")).expect("moves");
    std::os::unix::fs::symlink(&outside, root.join("out")).expect("links");
    let write = Write::Clean {
        files: clean.files,
        dirs: clean.dirs,
        ignored: false,
    };
    assert!(run(&root, write).is_err(), "a link in the path");
    assert!(outside.join("early.txt").exists());
}

#[test]
fn twin_reflog_events_stay_apart() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    for _ in 0..2 {
        let reset = std::process::Command::new(&git().path)
            .args(["reset", "-q", "--keep", "HEAD"])
            .current_dir(&root)
            .env("GIT_COMMITTER_DATE", "2030-01-01T00:00:00Z")
            .status()
            .expect("runs");
        assert!(reset.success());
    }
    let moves = recover::reflog(&git(), &root).expect("reads");
    assert_eq!((moves[0].at, &moves[0].said), (moves[1].at, &moves[1].said));
    assert!(!moves[0].same(&moves[1]), "two events, two identities");
    let picked = moves[1].clone();
    sh(&root, &["checkout", "-q", "--detach", "HEAD"]);
    let again = recover::reflog(&git(), &root).expect("reads");
    let found: Vec<usize> = (0..again.len())
        .filter(|ix| again[*ix].same(&picked))
        .collect();
    assert_eq!(found, [2], "it moved one down, alone");
}

#[test]
fn a_shown_file_deleted_since_says_it_changed() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("gone.txt"), "x").expect("writes");
    let clean = recover::clean_preview(&git(), &root, false).expect("previews");
    std::fs::remove_file(root.join("gone.txt")).expect("removes");
    let write = Write::Clean {
        files: clean.files,
        dirs: clean.dirs,
        ignored: false,
    };
    let error = run(&root, write).expect_err("refused");
    assert!(
        error
            .to_string()
            .contains("gone.txt changed since the preview"),
        "{error}"
    );
}

#[test]
fn a_pick_in_one_repository_stays_there() {
    let (one, other) = (
        tempfile::tempdir().expect("a temp dir"),
        tempfile::tempdir().expect("a temp dir"),
    );
    let (first, second) = (committed(one.path()), committed(other.path()));
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, one.path());
    for root in [&first, &second] {
        cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    }
    settle(cx, |app| {
        tree.read(app).repos.len() == 2 && branch(tree.read(app)).is_some()
    });
    let ids = cx.update(|_, app| {
        tree.read(app)
            .repos
            .iter()
            .map(|o| o.id)
            .collect::<Vec<_>>()
    });
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.activate(Some(ids[0]), cx);
            tree.pick_place(crate::places::Place::Remote("origin".into()), cx);
            tree.pick_ref(crate::branches::Kind::Tag, "v1".into(), cx);
            tree.activate(Some(ids[1]), cx);
        })
    });
    assert!(cx.update(|_, app| {
        tree.read(app)
            .active()
            .is_some_and(|o| o.picked_place.is_none() && o.picked_ref.is_none())
    }));
}
