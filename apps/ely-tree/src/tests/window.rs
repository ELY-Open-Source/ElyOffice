//! The window: tabs, and rereads after outside Git (TREE-01, TREE-02).

use std::time::{Duration, Instant};

use gpui::{App, Entity, TestAppContext, VisualTestContext};

use super::{committed, sh};
use crate::{status::State, tree::Tree};

/// Steps, lets the watcher's timer pass, and redraws until `done`.
pub(crate) fn settle(cx: &mut VisualTestContext, done: impl Fn(&App) -> bool) {
    settle_within(cx, Duration::from_secs(10), done)
}

pub(crate) fn settle_within(cx: &mut VisualTestContext, wait: Duration, done: impl Fn(&App) -> bool) {
    let deadline = Instant::now() + wait;
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
        cx.executor().advance_clock(Duration::from_millis(300));
        for _ in 0..100 {
            if !cx.executor().tick() {
                break;
            }
        }
        cx.update(|window, _| window.refresh());
        if cx.update(|_, app| done(app)) {
            return;
        }
    }
    panic!("the state never settled");
}

pub(crate) fn window<'a>(
    cx: &'a mut TestAppContext,
    data: &std::path::Path,
) -> (Entity<Tree>, &'a mut VisualTestContext) {
    cx.executor().allow_parking();
    cx.update(ely_gpui_component::init_for_tests);
    cx.update(|cx| {
        ely_ui::settings::install_defaults(cx);
        ely_ui::i18n::install_as("en-US", cx);
        // As tool_root does: menus before the window.
        let menus = crate::menu::menus(cx);
        let menus = ely_ui::commands::menus(ely_platform::AppId::Tree, &ely_ui::commands::OPENING, false, menus, cx);
        cx.set_menus(menus);
    });
    let recent = data.join("recent.json");
    cx.add_window_view(move |window, cx| Tree::new(recent, window, cx))
}

pub(crate) fn untracked(tree: &Tree) -> Vec<String> {
    let Some(Some(Ok(status))) = tree.active().map(|opened| &opened.status) else {
        return Vec::new();
    };
    status
        .entries
        .iter()
        .filter(|entry| entry.state == State::Untracked)
        .map(|entry| entry.path.clone())
        .collect()
}

pub(crate) fn branch(tree: &Tree) -> Option<String> {
    let Some(Some(Ok(status))) = tree.active().map(|opened| &opened.status) else {
        return None;
    };
    status.head.branch.clone()
}

#[test]
fn outside_git_and_edits_show_without_a_press() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    std::fs::write(root.join("new.txt"), "new\n").expect("writes");
    settle(cx, |app| untracked(tree.read(app)) == ["new.txt"]);
    sh(&root, &["checkout", "-q", "-b", "elsewhere"]);
    settle(cx, |app| {
        branch(tree.read(app)).as_deref() == Some("elsewhere")
    });
}

#[test]
fn one_tab_per_repository_and_reopening_selects_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (first, second) = (
        committed(&dir.path().join("1")),
        committed(&dir.path().join("2")),
    );
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    for (path, active) in [
        (first.clone(), 1),
        (second.clone(), 2),
        (first.join("."), 1),
    ] {
        cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(path, cx)));
        settle(cx, |app| tree.read(app).active == Some(active));
    }
    let tabs = cx.update(|_, app| tree.read(app).repos.len());
    assert_eq!(tabs, 2, "the same work tree is one tab");
    let recent = cx.update(|_, app| tree.read(app).recent.clone());
    assert_eq!(
        recent[0],
        dunce::canonicalize(&first).expect("real"),
        "newest first"
    );
    cx.update(|_, app| tree.update(app, |tree, cx| tree.close_active(cx)));
    assert_eq!(cx.update(|_, app| tree.read(app).active), Some(2));
    let missing = dir.path().join("gone");
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.recent.insert(0, missing.clone());
            tree.open_path(missing.clone(), cx)
        })
    });
    settle(cx, |app| !tree.read(app).recent.contains(&missing));
}

pub(crate) fn oid(tree: &Tree) -> Option<String> {
    let Some(Some(Ok(status))) = tree.active().map(|opened| &opened.status) else {
        return None;
    };
    status.head.oid.clone()
}

#[test]
fn a_linked_worktree_follows_refs_moved_from_the_main_one() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let main = committed(dir.path());
    let linked = dir.path().join("linked");
    sh(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "linked",
            linked.to_str().expect("utf-8"),
        ],
    );
    std::fs::write(main.join("b.txt"), "b\n").expect("writes");
    sh(&main, &["add", "b.txt"]);
    sh(&main, &["commit", "-q", "-m", "second"]);
    let second = sh(&main, &["rev-parse", "HEAD"]).trim().to_string();
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(linked.clone(), cx)));
    settle(cx, |app| {
        branch(tree.read(app)).as_deref() == Some("linked")
    });
    sh(&main, &["update-ref", "refs/heads/linked", &second]);
    settle(cx, |app| {
        oid(tree.read(app)).as_deref() == Some(second.as_str())
    });
}

#[test]
fn picked_lines_stage_from_the_window_and_the_picks_clear() {
    use ely_gpui_component::git::ChangeSection;
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("a.txt"), "one\nthree\nfour\n").expect("writes");
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.show("a.txt", ChangeSection::Unstaged, cx)
        })
    });
    let text = |app: &App| {
        matches!(
            tree.read(app).shown.as_ref().and_then(|s| s.view.as_ref()),
            Some(Ok(crate::shown::View::Text { .. }))
        )
    };
    settle(cx, text);
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.pick_line((0, 1), false, cx);
            tree.pick_line((0, 3), true, cx);
        })
    });
    let picked = cx.update(|_, app| tree.read(app).shown.as_ref().expect("shown").picked.clone());
    assert_eq!(
        picked.into_iter().collect::<Vec<_>>(),
        [(0, 1), (0, 2), (0, 3)],
        "Shift takes the range"
    );
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.pick_line((0, 1), false, cx);
            tree.pick_line((0, 2), false, cx);
            tree.stage_picked(cx)
        })
    });
    settle(cx, |_| {
        sh(&root, &["cat-file", "blob", ":a.txt"]) == "one\ntwo\nfour\n"
    });
    settle(cx, |app| {
        tree.read(app)
            .shown
            .as_ref()
            .is_some_and(|shown| shown.picked.is_empty())
    });
}

#[test]
fn a_late_read_of_another_file_is_dropped() {
    use ely_gpui_component::git::ChangeSection;
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("b.txt"), "before-b\n").expect("writes");
    sh(&root, &["add", "b.txt"]);
    sh(&root, &["commit", "-qm", "b"]);
    std::fs::write(root.join("a.txt"), "after-a\n").expect("writes");
    std::fs::write(root.join("b.txt"), "after-b\n").expect("writes");
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.show("a.txt", ChangeSection::Unstaged, cx)
        })
    });
    let first = cx.update(|_, app| tree.read(app).reads);
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.show("b.txt", ChangeSection::Unstaged, cx)
        })
    });
    let shows = |app: &App, word: &str| match tree
        .read(app)
        .shown
        .as_ref()
        .and_then(|shown| shown.view.as_ref())
    {
        Some(Ok(crate::shown::View::Text { diff, .. })) => format!("{diff:?}").contains(word),
        _ => false,
    };
    settle(cx, |app| shows(app, "after-b"));
    // a.txt's read lands after b.txt is shown.
    let late = crate::shown::read(
        &super::git(),
        &root,
        &super::entry(&root, "a.txt"),
        ChangeSection::Unstaged,
        crate::diff::Space::Exact,
        true,
    );
    cx.update(|_, app| tree.update(app, |tree, cx| tree.shown_read(first, late, cx)));
    assert!(
        cx.update(|_, app| shows(app, "after-b")),
        "b.txt stays shown"
    );
}

#[test]
fn a_commit_from_the_box_shows_in_history_with_its_files() {
    use crate::tree::Side;
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    for (key, value) in [
        ("user.name", "Ely"),
        ("user.email", "ely@example.com"),
        ("commit.gpgsign", "false"),
    ] {
        sh(&root, &["config", key, value]);
    }
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.set_side(Side::History, cx)));
    let rows = |app: &App| {
        tree.read(app)
            .active()
            .map_or(0, |opened| opened.hist.rows.len())
    };
    settle(cx, |app| rows(app) == 1);
    std::fs::write(root.join("a.txt"), "changed\n").expect("writes");
    sh(&root, &["add", "a.txt"]);
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.subject.update(cx, |field, cx| {
                field.set_text("From the box".to_string(), cx)
            });
            tree.commit(cx)
        })
    });
    // HEAD moved, so history rereads on its own.
    settle(cx, |app| rows(app) == 2);
    let newest = cx.update(|_, app| tree.read(app).active().expect("open").hist.rows[0].clone());
    assert_eq!(newest.subject.as_ref(), "From the box");
    assert!(
        cx.update(|_, app| tree.read(app).subject.read(app).text().is_empty()),
        "the box empties"
    );
    cx.update(|_, app| tree.update(app, |tree, cx| tree.pick_commit(&newest.id, cx)));
    let detail = |app: &App| {
        matches!(
            tree.read(app)
                .active()
                .and_then(|o| o.hist.pick.detail.as_ref()),
            Some(Ok(_))
        )
    };
    settle(cx, detail);
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.pick_touched(crate::hist::Whose::History, 0, cx)
        })
    });
    let file = |app: &App| {
        matches!(
            tree.read(app)
                .active()
                .and_then(|o| o.hist.pick.file.as_ref()),
            Some((0, Some(Ok(_))))
        )
    };
    settle(cx, file);
}

#[test]
fn any_new_pick_leaves_a_file_history() {
    use crate::tree::Side;
    use ely_gpui_component::git::ChangeSection;
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    let head = sh(&root, &["rev-parse", "HEAD"]).trim().to_string();
    let looking = |cx: &mut VisualTestContext| cx.update(|_, app| tree.read(app).look.is_some());
    type Pick = Box<dyn Fn(&mut Tree, &mut gpui::Context<Tree>)>;
    let picks: [(&str, Pick); 5] = [
        (
            "a tab",
            Box::new(|tree, cx| tree.set_side(Side::Branches, cx)),
        ),
        (
            "a commit",
            Box::new(move |tree, cx| tree.pick_commit(&head, cx)),
        ),
        (
            "a file",
            Box::new(|tree, cx| tree.show("a.txt", ChangeSection::Unstaged, cx)),
        ),
        (
            "a branch",
            Box::new(|tree, cx| tree.pick_ref(crate::branches::Kind::Local, "main".into(), cx)),
        ),
        (
            "a remote",
            Box::new(|tree, cx| tree.pick_place(crate::places::Place::Remote("origin".into()), cx)),
        ),
    ];
    for (what, pick) in picks {
        cx.update(|_, app| tree.update(app, |tree, cx| tree.file_log("a.txt", cx)));
        assert!(looking(cx), "the history opens");
        cx.update(|_, app| tree.update(app, |tree, cx| pick(tree, cx)));
        assert!(!looking(cx), "{what} leaves the history");
    }
}

#[test]
fn a_result_that_did_not_save_says_so() {
    use crate::work::Write;
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    // The window's toasts live on its root.
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
    let id = cx.update(|_, app| tree.read(app).active().expect("open").id);
    let save = Write::Save {
        path: "a.txt".into(),
        text: "merged".into(),
    };
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.written(
                id,
                &save,
                Err(std::io::Error::other("disk full")),
                false,
                cx,
            )
        })
    });
    let want =
        cx.update(|_, app| ely_ui::i18n::text(app, "tree.error.save", &[("error", "disk full")]));
    let toasts = cx.update(|_, app| shell.read(app).toasts(app));
    assert!(toasts.iter().any(|(text, _)| *text == want), "{toasts:?}");
}

#[test]
fn unstaging_right_after_a_first_commit_deletes_nothing() {
    use crate::work::Write;
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path().join("new");
    crate::repo::init(&super::git(), &root).expect("init");
    std::fs::write(root.join("a.txt"), "one\n").expect("writes");
    sh(&root, &["add", "a.txt"]);
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| {
        tree.read(app)
            .active()
            .is_some_and(|opened| matches!(opened.status, Some(Ok(_))))
    });
    let id = cx.update(|_, app| tree.read(app).active().expect("open").id);
    // Committed outside; the status has not read it yet.
    sh(&root, &["commit", "-qm", "first"]);
    cx.update(|_, app| tree.update(app, |tree, cx| tree.write(id, Write::UnstageAll, cx)));
    settle(cx, |app| {
        tree.read(app)
            .active()
            .is_some_and(|opened| !opened.writing && opened.writes.is_empty())
    });
    assert_eq!(
        sh(&root, &["status", "--porcelain"]),
        "",
        "nothing staged for deletion"
    );
}

#[test]
fn the_repository_menu_offers_every_tool() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (_tree, cx) = window(&mut cx, dir.path());
    let names = cx.update(|_, app| {
        crate::menu::menus(app)[0]
            .items
            .iter()
            .filter_map(|item| match item {
                gpui::MenuItem::Action { name, .. } => Some(name.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>()
    });
    let mut said = |key: &str| cx.update(|_, app| ely_ui::i18n::text(app, key, &[]).to_string());
    let (terminal, reveal, clone) = (
        said("tree.tool.terminal"),
        said(crate::tools::REVEAL),
        said("tree.clone"),
    );
    assert!(
        names.contains(&terminal) && names.contains(&reveal) && names.contains(&clone),
        "{names:?}"
    );
}
