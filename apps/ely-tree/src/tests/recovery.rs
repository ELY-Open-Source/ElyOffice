//! Reflog, moving back, cleaning, locks, failures (C13.6).

use std::path::Path;

use gpui::{App, Entity, TestAppContext};

use super::{
    branching::conflicted,
    committed, git,
    places::me,
    sh,
    window::{branch, settle, window},
};
use crate::{
    asks::Ask,
    ops::Op,
    recover,
    tree::Tree,
    work::{self, Write},
};

pub(super) fn run(root: &Path, write: Write) -> std::io::Result<()> {
    work::run(&git(), root, &write, &write.args(true))
}

/// Says yes to the dialog's write, as its button would.
fn confirm(tree: &Entity<Tree>, app: &mut App) {
    tree.update(app, |tree, cx| {
        let Some(Ask::Confirm { repo, write, .. }) = tree.ask.take() else {
            panic!("a confirm dialog");
        };
        tree.write(repo, write, cx);
    });
}

#[test]
fn the_reflog_lists_where_head_was_newest_first() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    me(&root);
    std::fs::write(root.join("a.txt"), "two\n").expect("writes");
    sh(&root, &["commit", "-qam", "second"]);
    sh(&root, &["checkout", "-q", "--detach", "HEAD~1"]);
    let moves = recover::reflog(&git(), &root).expect("reads");
    let names: Vec<&str> = moves.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["HEAD@{0}", "HEAD@{1}", "HEAD@{2}"]);
    assert!(moves[0].said.starts_with("checkout:"), "{}", moves[0].said);
    assert!(moves[1].said.ends_with("second"), "{}", moves[1].said);
}

fn move_of(plan: &recover::Move) -> Write {
    Write::Move {
        branch: plan.branch.clone(),
        head: plan.head.clone(),
        to: plan.to.clone(),
    }
}

#[test]
fn moving_back_keeps_changes_and_refuses_a_moved_head() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    me(&root);
    let first = sh(&root, &["rev-parse", "HEAD"]).trim().to_string();
    std::fs::write(root.join("b.txt"), "b\n").expect("writes");
    sh(&root, &["add", "b.txt"]);
    sh(&root, &["commit", "-qm", "gone soon"]);
    std::fs::write(root.join("a.txt"), "one\ntwo\nkept\n").expect("writes");
    let plan = recover::plan_move(&git(), &root, &first).expect("a plan");
    assert_eq!(plan.left.len(), 1);
    assert!(plan.left[0].ends_with("gone soon"));
    sh(&root, &["commit", "-q", "--allow-empty", "-m", "meanwhile"]);
    let refused = run(&root, move_of(&plan)).expect_err("HEAD moved");
    assert!(refused.to_string().contains("moved"), "{refused}");
    sh(&root, &["reset", "-q", "--keep", "HEAD~1"]);
    run(&root, move_of(&plan)).expect("moves");
    assert_eq!(sh(&root, &["rev-parse", "HEAD"]).trim(), first);
    assert_eq!(
        std::fs::read_to_string(root.join("a.txt")).expect("reads"),
        "one\ntwo\nkept\n"
    );
    assert!(!root.join("b.txt").exists());
}

#[test]
fn staged_work_refuses_a_move_rather_than_lose_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    me(&root);
    let first = sh(&root, &["rev-parse", "HEAD"]).trim().to_string();
    sh(&root, &["commit", "-q", "--allow-empty", "-m", "second"]);
    std::fs::write(root.join("a.txt"), "staged\n").expect("writes");
    sh(&root, &["add", "a.txt"]);
    std::fs::write(root.join("a.txt"), "unstaged\n").expect("writes");
    assert!(
        recover::plan_move(&git(), &root, &first).is_err(),
        "no plan"
    );
    // Staged after the plan: the write refuses too.
    sh(&root, &["reset", "-q"]);
    let plan = recover::plan_move(&git(), &root, &first).expect("a plan");
    sh(&root, &["add", "a.txt"]);
    let refused = run(&root, move_of(&plan)).expect_err("staged");
    assert!(refused.to_string().contains("staged"), "{refused}");
    assert_eq!(sh(&root, &["show", ":a.txt"]), "unstaged\n", "index kept");
}

#[test]
fn a_move_refuses_another_branch_or_a_detached_head() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    me(&root);
    let first = sh(&root, &["rev-parse", "HEAD"]).trim().to_string();
    sh(&root, &["commit", "-q", "--allow-empty", "-m", "second"]);
    let second = sh(&root, &["rev-parse", "HEAD"]).trim().to_string();
    sh(&root, &["branch", "other"]);
    let plan = recover::plan_move(&git(), &root, &first).expect("a plan");
    sh(&root, &["switch", "-q", "other"]);
    assert!(
        run(&root, move_of(&plan)).is_err(),
        "same commit, other branch"
    );
    sh(&root, &["switch", "-q", "--detach", &second]);
    assert!(run(&root, move_of(&plan)).is_err(), "detached now");
    for name in ["other", "HEAD"] {
        assert_eq!(sh(&root, &["rev-parse", name]).trim(), second);
    }
}

#[test]
fn a_held_lock_waits_for_a_retry_and_stays() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("a.txt"), "one\nstaged\n").expect("writes");
    let lock = root.join(".git/index.lock");
    std::fs::write(&lock, "").expect("writes");
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    let id = cx.update(|_, app| tree.read(app).active.expect("open"));
    let entry = super::entry(&root, "a.txt");
    cx.update(|_, app| tree.update(app, |tree, cx| tree.write(id, Write::Stage(entry), cx)));
    settle(cx, |app| {
        matches!(tree.read(app).ask, Some(Ask::Locked { .. }))
    });
    assert!(lock.exists(), "Ely deletes no lock");
    std::fs::remove_file(&lock).expect("the other Git ends");
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            let Some(Ask::Locked { repo, write, .. }) = tree.ask.take() else {
                panic!("locked");
            };
            tree.write(repo, write, cx);
        })
    });
    settle(cx, |_| {
        sh(&root, &["diff", "--cached", "--name-only"]).trim() == "a.txt"
    });
}

#[test]
fn a_refused_credential_says_so() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    // A server that wants a password nobody gives.
    let server = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
    let url = format!(
        "http://{}/repo.git",
        server.local_addr().expect("an address")
    );
    std::thread::spawn(move || {
        for stream in server.incoming().flatten() {
            use std::io::{Read as _, Write as _};
            let mut stream = stream;
            let mut buffer = [0u8; 4096];
            let _ = stream.read(&mut buffer);
            let _ = stream.write_all(
                b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"x\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        }
    });
    sh(&root, &["remote", "add", "origin", &url]);
    sh(&root, &["config", "credential.helper", ""]);
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.fetch_one("origin", cx)));
    settle(
        cx,
        |app| matches!(&tree.read(app).ask, Some(Ask::Failed { detail, .. }) if detail.contains("terminal prompts disabled")),
    );
}

#[test]
fn an_interrupted_rebase_aborts_back_to_where_it_was() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (root, _) = conflicted(dir.path());
    let before = sh(&root, &["rev-parse", "HEAD"]);
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.rebase("refs/heads/side", "side", cx)));
    cx.update(|_, app| confirm(&tree, app));
    let doing = |app: &App, op: Option<Op>| {
        matches!(
            tree.read(app).active().and_then(|o| o.doing.clone()),
            Some(Ok(now)) if now.as_ref().map(|d| d.op) == op
        )
    };
    settle(cx, |app| doing(app, Some(Op::Rebase)));
    assert!(
        cx.update(|_, app| tree.read(app).ask.is_none()),
        "a stop, no failure"
    );
    cx.update(|_, app| tree.update(app, |tree, cx| tree.op_abort(Op::Rebase, cx)));
    cx.update(|_, app| confirm(&tree, app));
    settle(cx, |app| doing(app, None));
    assert_eq!(sh(&root, &["rev-parse", "HEAD"]), before);
    assert!(sh(&root, &["status", "--porcelain"]).trim().is_empty());
}

#[test]
fn a_reflog_entry_shows_and_moves_the_branch_back() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    me(&root);
    let first = sh(&root, &["rev-parse", "HEAD"]).trim().to_string();
    sh(&root, &["commit", "-q", "--allow-empty", "-m", "later"]);
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.set_side(crate::tree::Side::Places, cx)));
    settle(cx, |app| {
        tree.read(app)
            .active()
            .and_then(|o| o.places.read.as_ref())
            .is_some_and(|read| matches!(&read.reflog, Ok(moves) if moves.len() == 2))
    });
    let place = cx.update(|_, app| reflog_at(&tree, app, 1));
    cx.update(|_, app| tree.update(app, |tree, cx| tree.pick_place(place, cx)));
    settle(cx, |app| {
        matches!(
            tree.read(app).active().and_then(|o| o.places.pick.detail.as_ref()),
            Some(Ok(detail)) if detail.id == first
        )
    });
    cx.update(|_, app| tree.update(app, |tree, cx| tree.ask_move(&first, cx)));
    settle(
        cx,
        |app| matches!(&tree.read(app).ask, Some(Ask::Move { plan, .. }) if plan.left.len() == 1),
    );
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            let Some(Ask::Move { repo, plan }) = tree.ask.take() else {
                panic!("a move");
            };
            tree.write(repo, move_of(&plan), cx);
        })
    });
    settle(cx, |_| sh(&root, &["rev-parse", "HEAD"]).trim() == first);
}

#[test]
fn the_reflog_says_when_head_moved_not_when_committed() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path().join("work");
    crate::repo::init(&git(), &root).expect("init");
    let old = std::process::Command::new(&git().path)
        .args([
            "-c",
            "user.name=Ely",
            "-c",
            "user.email=e@x",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "old",
        ])
        .current_dir(&root)
        .env("GIT_COMMITTER_DATE", "2000-01-01T00:00:00Z")
        .env("GIT_AUTHOR_DATE", "2000-01-01T00:00:00Z")
        .status()
        .expect("runs");
    assert!(old.success());
    sh(&root, &["checkout", "-q", "-b", "other"]);
    let moves = recover::reflog(&git(), &root).expect("reads");
    assert!(
        moves[0].at > 1_700_000_000,
        "the checkout's own time: {}",
        moves[0].at
    );
}

/// The reflog event at `at`, as picked.
fn reflog_at(tree: &Entity<Tree>, app: &App, at: usize) -> crate::places::Place {
    let read = tree.read(app).active().and_then(|o| o.places.read.as_ref());
    let Some(crate::places::Read {
        reflog: Ok(moves), ..
    }) = read
    else {
        panic!("a reflog");
    };
    crate::places::Place::Reflog(moves[at].clone())
}

#[test]
fn a_picked_reflog_event_stays_itself_as_others_arrive() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    sh(&root, &["checkout", "-q", "-b", "other"]);
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.set_side(crate::tree::Side::Places, cx)));
    let count = |app: &App| match tree.read(app).active().and_then(|o| o.places.read.as_ref()) {
        Some(crate::places::Read {
            reflog: Ok(moves), ..
        }) => moves.len(),
        _ => 0,
    };
    settle(cx, |app| count(app) == 2);
    // The initial commit and the checkout share one commit.
    let picked = cx.update(|_, app| reflog_at(&tree, app, 1));
    let newer = cx.update(|_, app| reflog_at(&tree, app, 0));
    cx.update(|_, app| tree.update(app, |tree, cx| tree.pick_place(picked.clone(), cx)));
    let crate::places::Place::Reflog(was) = &picked else {
        panic!("a reflog event");
    };
    let crate::places::Place::Reflog(new) = &newer else {
        panic!("a reflog event");
    };
    assert!(!was.same(new), "one commit, two events");
    sh(&root, &["checkout", "-q", "--detach", "HEAD"]);
    settle(cx, |app| count(app) == 3);
    let now = cx.update(|_, app| reflog_at(&tree, app, 2));
    let crate::places::Place::Reflog(now) = now else {
        panic!("a reflog event");
    };
    assert!(now.same(was), "the event moved to HEAD@{{2}}");
    assert!(cx.update(|_, app| {
        tree.read(app)
            .active()
            .and_then(|o| o.picked_place.as_ref())
            == Some(&picked)
    }));
}

#[test]
fn a_held_config_lock_waits_for_a_retry_too() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    let lock = root.join(".git/config.lock");
    std::fs::write(&lock, "").expect("writes");
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    let id = cx.update(|_, app| tree.read(app).active.expect("open"));
    let write = Write::Config {
        key: "user.name".into(),
        value: "Locked Out".into(),
        global: false,
    };
    cx.update(|_, app| tree.update(app, |tree, cx| tree.write(id, write, cx)));
    settle(cx, |app| {
        matches!(tree.read(app).ask, Some(Ask::Locked { .. }))
    });
    assert!(lock.exists());
    std::fs::remove_file(&lock).expect("the other Git ends");
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            let Some(Ask::Locked { repo, write, .. }) = tree.ask.take() else {
                panic!("locked");
            };
            tree.write(repo, write, cx);
        })
    });
    settle(cx, |_| {
        let read = git()
            .run(&root, &["config", "--get", "user.name"])
            .expect("reads");
        String::from_utf8_lossy(&read).trim() == "Locked Out"
    });
}

#[test]
fn a_read_that_lands_late_leaves_the_users_dialog_open() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    let head = sh(&root, &["rev-parse", "HEAD"]).trim().to_string();
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
    let missing = "0123456789abcdef0123456789abcdef01234567";
    let said = recover::plan_move(&git(), &root, missing).expect_err("no such commit");
    for to in [head.as_str(), missing] {
        cx.update(|_, app| {
            tree.update(app, |tree, cx| {
                tree.ask_move(to, cx);
                tree.ask = Some(Ask::Failed {
                    title: "mine".into(),
                    detail: "".into(),
                });
            })
        });
        cx.run_until_parked();
        let ask = cx.update(|_, app| match &tree.read(app).ask {
            Some(Ask::Failed { title, .. }) => title.to_string(),
            _ => "replaced".into(),
        });
        assert_eq!(ask, "mine", "{to}");
    }
    // A failed read still says why, as a toast.
    let title = cx.update(|_, app| ely_ui::i18n::text(app, "tree.op.failed", &[]));
    let want = format!("{title}: {said}");
    let toasts = cx.update(|_, app| shell.read(app).toasts(app));
    let danger = Some(ely_gpui_component::primitives::Severity::Danger);
    assert!(toasts.iter().any(|(text, severity)| *text == want && *severity == danger), "{toasts:?}");
}
