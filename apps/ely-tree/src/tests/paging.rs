//! History pages against refs that move between them (C13.3).

use std::{collections::HashSet, io::Write, path::Path, process::Stdio};

use gpui::{App, TestAppContext};

use super::{
    committed, git, sh,
    window::{branch, settle, window},
};
use crate::{commits::PAGE, tree::Side};

/// `count` empty commits on top of HEAD, made at once.
fn many(root: &Path, count: usize) {
    let head = sh(root, &["rev-parse", "HEAD"]).trim().to_string();
    let main = sh(root, &["symbolic-ref", "HEAD"]).trim().to_string();
    let mut stream = String::new();
    for n in 0..count {
        let message = format!("page {n}\n");
        stream += &format!("commit {main}\ncommitter Ely <ely@example.com> {} +0000\n", 1_700_000_000 + n);
        stream += &format!("data {}\n{message}", message.len());
        if n == 0 {
            stream += &format!("from {head}\n");
        }
        stream += "\n";
    }
    let mut child = ely_platform::git::command(&git().path, root, &["fast-import", "--quiet"])
        .stdin(Stdio::piped())
        .spawn()
        .expect("fast-import starts");
    child.stdin.take().expect("stdin").write_all(stream.as_bytes()).expect("writes");
    assert!(child.wait().expect("fast-import ends").success());
}

#[test]
fn a_commit_landing_between_pages_rereads_from_the_top() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    many(&root, PAGE);
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.set_side(Side::History, cx)));
    let rows = |app: &App| tree.read(app).active().map(|o| o.hist.rows.clone()).unwrap_or_default();
    settle(cx, |app| rows(app).len() == PAGE);
    // A terminal commits; the next page starts before the watcher.
    sh(&root, &["commit", "-q", "--allow-empty", "-m", "outside"]);
    cx.update(|_, app| tree.update(app, |tree, cx| tree.more_log(cx)));
    settle(cx, |app| rows(app).first().is_some_and(|row| row.subject.as_ref() == "outside"));
    let shown = cx.update(|_, app| rows(app));
    let ids: HashSet<_> = shown.iter().map(|row| row.id.clone()).collect();
    assert_eq!(ids.len(), shown.len(), "no commit shows twice");
}
