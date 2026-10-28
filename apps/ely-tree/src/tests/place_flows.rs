//! The Repository panel in the window (C13.5).

use ely_gpui_component::git::StashAction;
use gpui::{App, Entity, TestAppContext};

use super::{
    committed,
    places::me,
    sh,
    window::{branch, settle, settle_within, window},
};
use crate::{
    asks::Ask,
    places::Place,
    stash::Stash,
    tree::{Side, Tree},
};

fn stashes(tree: &Tree) -> Vec<Stash> {
    tree.stashes().to_vec()
}

fn running(tree: &Entity<Tree>, app: &App) -> bool {
    tree.read(app)
        .active()
        .is_some_and(|opened| opened.net.is_some())
}

#[test]
fn a_stash_is_made_shown_and_popped_from_the_window() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    me(&root);
    std::fs::write(root.join("a.txt"), "one\nkept\n").expect("writes");
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.set_side(Side::Places, cx)));
    settle(cx, |app| {
        tree.read(app)
            .active()
            .is_some_and(|o| o.places.read.is_some())
    });
    cx.update(|window, app| tree.update(app, |tree, cx| tree.ask_stash(window, cx)));
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            if let Some(Ask::Stash { message, .. }) = &tree.ask {
                message.update(cx, |field, cx| field.set_text("kept for later", cx));
            }
            tree.answer(cx);
        })
    });
    settle(cx, |app| stashes(tree.read(app)).len() == 1);
    let stash = cx.update(|_, app| stashes(tree.read(app))[0].clone());
    assert_eq!(stash.message, "kept for later");
    assert_eq!(
        std::fs::read_to_string(root.join("a.txt")).expect("reads"),
        "one\ntwo\n"
    );
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.pick_place(Place::Stash(stash.oid.clone()), cx)
        })
    });
    settle(cx, |app| {
        matches!(
            tree.read(app).active().and_then(|o| o.places.pick.detail.as_ref()),
            Some(Ok(detail)) if detail.files.iter().any(|f| f.path == "a.txt")
        )
    });
    cx.update(|_, app| tree.update(app, |tree, cx| tree.stash_act(0, StashAction::Drop, cx)));
    assert!(
        cx.update(|_, app| matches!(tree.read(app).ask, Some(Ask::Confirm { .. }))),
        "a drop asks"
    );
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.ask = None;
            tree.stash_act(0, StashAction::Pop, cx)
        })
    });
    settle(cx, |app| stashes(tree.read(app)).is_empty());
    assert_eq!(
        std::fs::read_to_string(root.join("a.txt")).expect("reads"),
        "one\nkept\n"
    );
}

/// A bare remote and a clone of it.
fn cloned(dir: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let source = committed(&dir.join("source"));
    let bare = dir.join("remote.git");
    let (from, to) = (
        source.to_str().expect("utf-8"),
        bare.to_str().expect("utf-8"),
    );
    sh(dir, &["clone", "-q", "--bare", from, to]);
    let into = dir.join("copy");
    sh(dir, &["clone", "-q", to, into.to_str().expect("utf-8")]);
    me(&into);
    (bare, into)
}

#[test]
fn fetch_runs_with_progress_and_lands() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (bare, into) = cloned(dir.path());
    let other = dir.path().join("other");
    sh(
        dir.path(),
        &[
            "clone",
            "-q",
            bare.to_str().expect("utf-8"),
            other.to_str().expect("utf-8"),
        ],
    );
    std::fs::write(other.join("a.txt"), "moved\n").expect("writes");
    sh(&other, &["commit", "-qam", "moved"]);
    sh(&other, &["push", "-q"]);
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(into.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    let name = cx.update(|_, app| branch(tree.read(app)).expect("a branch"));
    cx.update(|_, app| tree.update(app, |tree, cx| tree.fetch_all(cx)));
    let tracked = format!("refs/remotes/origin/{name}");
    let moved = sh(&other, &["rev-parse", "HEAD"]);
    settle(cx, |app| {
        !running(&tree, app) && sh(&into, &["rev-parse", &tracked]) == moved
    });
    assert!(cx.update(|_, app| tree.read(app).ask.is_none()));
}

#[test]
fn a_stopped_fetch_ends_quietly() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    let silent = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
    let url = format!("git://{}/nothing", silent.local_addr().expect("an address"));
    sh(&root, &["remote", "add", "origin", &url]);
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.fetch_one("origin", cx)));
    settle(cx, |app| running(&tree, app));
    let _held = connected(&silent);
    let id = cx.update(|_, app| tree.read(app).active.expect("open"));
    cx.update(|_, app| tree.update(app, |tree, cx| tree.stop_remote(id, cx)));
    settle_within(cx, GRACE, |app| !running(&tree, app));
    assert!(
        cx.update(|_, app| tree.read(app).ask.is_none()),
        "a stop is no failure"
    );
}

#[test]
fn a_stopped_clone_leaves_no_folder() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let silent = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
    let url = format!("git://{}/nothing", silent.local_addr().expect("an address"));
    let into = dir.path().join("into");
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.start_clone(url, into.clone(), cx)));
    let _held = connected(&silent);
    settle(cx, |_| into.exists());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.stop_clone(cx)));
    settle_within(cx, GRACE, |app| tree.read(app).cloning.is_none());
    assert!(!into.exists(), "Git removed the clone it began");
    assert!(cx.update(|_, app| tree.read(app).ask.is_none()), "a stop is no failure");
}

/// Git's connection; a Git that never starts fails.
fn connected(silent: &std::net::TcpListener) -> std::net::TcpStream {
    silent.set_nonblocking(true).expect("nonblocking");
    let start = std::time::Instant::now();
    loop {
        match silent.accept() {
            Ok((stream, _)) => return stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(start.elapsed() < GRACE, "Git never connected");
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(error) => panic!("accept: {error}"),
        }
    }
}

/// Windows may end a hung Git only after 10 s.
const GRACE: std::time::Duration = std::time::Duration::from_secs(30);

/// Git outside its own session; no group stop reaches it.
#[cfg(unix)]
fn unstoppable() -> std::process::Child {
    std::process::Command::new(&super::git().path)
        .args(["cat-file", "--batch"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("Git starts")
}

#[cfg(unix)]
#[test]
fn a_stop_that_fails_names_git_not_the_clone() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    let silent = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
    let url = format!("git://{}/nothing", silent.local_addr().expect("an address"));
    sh(&root, &["remote", "add", "origin", &url]);
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    // The window's toasts live on its root.
    let shell = cx.update(|window, app| {
        let shell =
            ely_ui::tool_root(ely_platform::AppId::Tree, tree.clone().into(), Vec::new(), window, app);
        tree.update(app, |tree, _| tree.root = shell.downgrade());
        shell
    });
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.fetch_one("origin", cx)));
    settle(cx, |app| running(&tree, app));
    let held = connected(&silent);
    let id = cx.update(|_, app| tree.read(app).active.expect("open"));
    let stop = crate::running::Stop::over(unstoppable());
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            let net = tree.opened_mut(id).and_then(|opened| opened.net.as_mut()).expect("a run");
            let real = std::mem::replace(&mut net.stop, stop.clone());
            tree.stop_remote(id, cx);
            tree.opened_mut(id).and_then(|opened| opened.net.as_mut()).expect("a run").stop = real;
        })
    });
    let said = cx.update(|_, app| ely_ui::i18n::text(app, "tree.error.stop_remote", &[("error", "")]));
    let toasts = cx.update(|_, app| shell.read(app).toasts(app));
    assert!(toasts.iter().any(|(text, _)| text.starts_with(said.as_ref())), "{toasts:?}");
    drop(held);
    settle(cx, |app| !running(&tree, app));
    // The fetch's own failure still shows; no stop is claimed.
    let stopped = cx.update(|_, app| ely_ui::i18n::text(app, "tree.net.stopped", &[]));
    let toasts = cx.update(|_, app| shell.read(app).toasts(app));
    assert!(toasts.iter().all(|(text, _)| *text != stopped), "{toasts:?}");
    assert!(
        cx.update(|_, app| matches!(tree.read(app).ask, Some(Ask::Failed { .. }))),
        "the fetch says why it failed"
    );
}

#[test]
fn a_push_with_no_upstream_asks_where_then_tracks_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (bare, into) = cloned(dir.path());
    sh(&into, &["switch", "-q", "-c", "topic"]);
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(into.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).as_deref() == Some("topic"));
    cx.update(|_, app| tree.update(app, |tree, cx| tree.push(cx)));
    settle(
        cx,
        |app| matches!(&tree.read(app).ask, Some(Ask::Choose { choices, .. }) if choices == &["origin"]),
    );
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.ask = None;
            tree.publish(tree.active.expect("open"), "topic", "origin", cx)
        })
    });
    settle(cx, |app| {
        !running(&tree, app)
            && sh(&into, &["config", "--get", "branch.topic.remote"]).trim() == "origin"
    });
    assert_eq!(
        sh(&bare, &["rev-parse", "topic"]),
        sh(&into, &["rev-parse", "HEAD"])
    );
}

#[test]
fn a_forced_push_shows_what_it_would_overwrite() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (bare, into) = cloned(dir.path());
    let other = dir.path().join("other");
    sh(
        dir.path(),
        &[
            "clone",
            "-q",
            bare.to_str().expect("utf-8"),
            other.to_str().expect("utf-8"),
        ],
    );
    std::fs::write(other.join("a.txt"), "theirs\n").expect("writes");
    sh(&other, &["commit", "-qam", "theirs"]);
    sh(&other, &["push", "-q"]);
    sh(&into, &["fetch", "-q"]);
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(into.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.ask_force(cx)));
    settle(
        cx,
        |app| matches!(&tree.read(app).ask, Some(Ask::Force { plan, .. }) if plan.lost.len() == 1),
    );
}

#[test]
fn an_upstream_is_set_and_unset_from_the_branches() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (_, into) = cloned(dir.path());
    let main = sh(&into, &["branch", "--show-current"]).trim().to_string();
    sh(&into, &["branch", "topic"]);
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(into.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    let remote = format!("origin/{main}");
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.set_upstream(tree.active.expect("open"), "topic", &remote, cx)
        })
    });
    let merge = || {
        sh(
            &into,
            &["config", "--default", "", "--get", "branch.topic.merge"],
        )
    };
    settle(cx, |_| merge().trim() == format!("refs/heads/{main}"));
    cx.update(|_, app| tree.update(app, |tree, cx| tree.unset_upstream("topic", cx)));
    settle(cx, |_| merge().trim().is_empty());
}

#[test]
fn a_dialog_answers_for_the_repository_it_opened_on() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let a = committed(&dir.path().join("a"));
    let b = committed(&dir.path().join("b"));
    for root in [&a, &b] {
        me(root);
        std::fs::write(root.join("a.txt"), "one\nchanged\n").expect("writes");
    }
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(a.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    let first = cx.update(|_, app| tree.read(app).active.expect("open"));
    cx.update(|window, app| {
        tree.update(app, |tree, cx| {
            tree.open_path(b.clone(), cx);
            tree.ask_stash(window, cx)
        })
    });
    settle(cx, |app| tree.read(app).active != Some(first));
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            if let Some(Ask::Stash { message, .. }) = &tree.ask {
                message.update(cx, |field, cx| field.set_text("for A", cx));
            }
            tree.answer(cx);
        })
    });
    let count = |root: &std::path::Path| {
        crate::stash::list(&super::git(), root)
            .expect("reads")
            .len()
    };
    settle(cx, |_| count(&a) == 1);
    assert_eq!(count(&b), 0, "B keeps its change");
}
