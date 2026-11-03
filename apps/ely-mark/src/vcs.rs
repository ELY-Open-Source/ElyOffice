//! Whether Git sees the file changed; ElyTree does the rest.

use std::{io, path::Path};

use ely_gpui_component::{primitives::Severity, shell::StatusBarItem};
use ely_office::prelude::*;
use ely_platform::{AppId, git::NoGit};
use ely_ui::i18n::text;
use gpui::Context;

use crate::desk::Desk;

/// What Git says of one file.
#[derive(Clone, Debug, PartialEq)]
pub enum Tracked {
    /// Not in a repository: nothing to show.
    Outside,
    Unchanged,
    Changed,
    Untracked,
    Ignored,
    Conflicted,
    /// Ely's Git sits beside ElyTree, which is not installed.
    NoGit,
    /// Git could not answer; why.
    Unknown(String),
}

/// Git's word on `file`: in a repository, then its status.
pub fn ask(file: &Path) -> Tracked {
    status(file).unwrap_or_else(failed)
}

/// A failure: no Git at all, else why.
fn failed(error: io::Error) -> Tracked {
    match error.get_ref().is_some_and(|inner| inner.is::<NoGit>()) {
        true => Tracked::NoGit,
        false => Tracked::Unknown(error.to_string()),
    }
}

fn run(git: &Path, dir: &Path, args: &[&str]) -> io::Result<std::process::Output> {
    ely_platform::git::command(git, dir, args).output()
}

fn status(file: &Path) -> io::Result<Tracked> {
    let (git, _) = ely_platform::git::locate()?;
    let dir = file
        .parent()
        .ok_or_else(|| io::Error::other("a file without a folder"))?;
    let top = run(&git, dir, &["rev-parse", "--show-toplevel"])?;
    let said = String::from_utf8_lossy(&top.stderr);
    // Not a repository is an answer; other failures are errors.
    if !top.status.success() {
        return match said.contains("not a git repository") {
            true => Ok(Tracked::Outside),
            false => Err(io::Error::other(said.trim().to_owned())),
        };
    }
    let name = file
        .file_name()
        .ok_or_else(|| io::Error::other("a file without a name"))?;
    let name = name.to_string_lossy();
    let said = run(
        &git,
        dir,
        &["status", "--porcelain=v2", "-z", "--ignored", "--", &name],
    )?;
    if !said.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&said.stderr).trim().to_owned(),
        ));
    }
    Ok(match said.stdout.first() {
        None => Tracked::Unchanged,
        Some(b'1' | b'2') => Tracked::Changed,
        Some(b'u') => Tracked::Conflicted,
        Some(b'?') => Tracked::Untracked,
        Some(b'!') => Tracked::Ignored,
        Some(other) => {
            return Err(io::Error::other(format!(
                "git status said {:?}",
                *other as char
            )));
        }
    })
}

impl Desk {
    /// Asks Git about the shown file, off the main thread.
    pub fn ask_git(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let Some(path) = tab.session.path().map(Path::to_path_buf) else {
            self.tracked = None;
            return;
        };
        let id = tab.id;
        cx.spawn(async move |desk, cx| {
            let asked = path.clone();
            let tracked = cx
                .background_executor()
                .spawn(async move { ask(&asked) })
                .await;
            desk.update(cx, |desk, cx| {
                // Only the shown file's answer counts.
                let shown = desk.active_tab().filter(|tab| tab.id == id);
                if shown.and_then(|tab| tab.session.path()) == Some(&path) {
                    log::info!("mark: git says {tracked:?}");
                    desk.tracked = Some(tracked);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// The Git status item; pressing shows the file in ElyTree.
    pub fn git_item(&self, cx: &mut Context<Self>) -> Option<StatusBarItem> {
        let key = match self.tracked.as_ref()? {
            Tracked::Outside => return None,
            Tracked::Unchanged => "mark.git.unchanged",
            Tracked::Changed => "mark.git.changed",
            Tracked::Untracked => "mark.git.untracked",
            Tracked::Ignored => "mark.git.ignored",
            Tracked::Conflicted => "mark.git.conflicted",
            Tracked::NoGit => {
                let this = cx.entity();
                let item = StatusBarItem::new("mark-git")
                    .label(text(cx, "mark.git.none", &[]))
                    .tooltip(text(cx, "mark.git.needs", &[]));
                return Some(item.on_click(move |_, _, cx| {
                    let said = text(cx, "mark.git.needs", &[]);
                    this.update(cx, |desk, cx| desk.toast(said, Severity::Warning, cx));
                }));
            }
            Tracked::Unknown(why) => {
                let label = text(cx, "mark.git.unknown", &[]);
                return Some(
                    StatusBarItem::new("mark-git")
                        .label(label)
                        .tooltip(why.clone()),
                );
            }
        };
        // ElyTree takes the file: its repository and its history (§16).
        let file = self.active_tab()?.session.path()?.to_path_buf();
        let item = StatusBarItem::new("mark-git")
            .label(text(cx, key, &[]))
            .tooltip(text(cx, "mark.git.open", &[]));
        let this = cx.entity();
        Some(item.on_click(move |_, _, cx| {
            if !ely_platform::apps::installed(AppId::Tree) {
                let said = text(cx, "mark.git.missing", &[]);
                this.update(cx, |desk, cx| desk.toast(said, Severity::Warning, cx));
                return;
            }
            let (this, file) = (this.clone(), file.clone());
            cx.spawn(async move |cx| {
                let opened = cx
                    .background_executor()
                    .spawn(async move { ely_platform::apps::open(AppId::Tree, &[file]) })
                    .await;
                if let Err(error) = opened {
                    // Logged even if the window has closed.
                    log::error!("mark: ElyTree did not open: {}", ely_ipc::Worded::logged(&error));
                    cx.update(|cx| this.update(cx, |desk, cx| desk.tree_failed(&error, cx)));
                }
            })
            .detach();
        }))
    }

    /// ElyTree did not start: its words; the caller logs it.
    pub(crate) fn tree_failed(&self, error: &io::Error, cx: &mut Context<Self>) {
        let said = text(cx, "mark.git.failed", &[("error", &ely_ui::i18n::error(cx, error))]);
        self.toast_unlogged(said, Severity::Danger, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        let (git, _) = ely_platform::git::locate().expect("a Git");
        let done = std::process::Command::new(git)
            .args(args)
            .current_dir(dir)
            .output();
        assert!(done.expect("git runs").status.success(), "git {args:?}");
    }

    #[test]
    fn no_git_asks_for_elytree_and_other_failures_say_why() {
        let missing = io::Error::new(io::ErrorKind::NotFound, NoGit);
        assert_eq!(failed(missing), Tracked::NoGit);
        let other = io::Error::new(io::ErrorKind::NotFound, "gone");
        assert_eq!(failed(other), Tracked::Unknown("gone".into()));
    }

    #[test]
    fn spaces_stay_and_broken_repositories_say_why() {
        let dir = tempfile::tempdir().expect("a temp dir");
        // Windows names end in no space.
        let repo = dir
            .path()
            .join(if cfg!(windows) { "re po" } else { "repo " });
        std::fs::create_dir(&repo).expect("the folder");
        git(&repo, &["init", "-q"]);
        std::fs::write(repo.join("t.md"), "a").expect("a file");
        assert_eq!(ask(&repo.join("t.md")), Tracked::Untracked);
        std::fs::write(repo.join(".git/config"), "[bad\n").expect("a broken config");
        assert!(matches!(ask(&repo.join("t.md")), Tracked::Unknown(_)));
        let outside = tempfile::tempdir().expect("a temp dir");
        std::fs::write(outside.path().join("t.md"), "a").expect("a file");
        assert_eq!(ask(&outside.path().join("t.md")), Tracked::Outside);
    }

    #[test]
    fn committed_changed_and_ignored_files_say_so() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let repo = dir.path();
        git(repo, &["init", "-q"]);
        std::fs::write(
            repo.join(".gitignore"),
            "skip.md
",
        )
        .expect("ignores");
        std::fs::write(repo.join("t.md"), "a").expect("a file");
        git(repo, &["add", "."]);
        git(
            repo,
            &[
                "-c",
                "user.name=T",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-qm",
                "one",
            ],
        );
        assert_eq!(ask(&repo.join("t.md")), Tracked::Unchanged);
        std::fs::write(repo.join("t.md"), "b").expect("an edit");
        assert_eq!(ask(&repo.join("t.md")), Tracked::Changed);
        std::fs::write(repo.join("skip.md"), "c").expect("a file");
        assert_eq!(ask(&repo.join("skip.md")), Tracked::Ignored);
    }

    #[gpui::test]
    fn the_status_item_follows_a_save(cx: &mut gpui::TestAppContext) {
        use ely_office::prelude::*;
        let (data, dir) = (
            tempfile::tempdir().expect("data"),
            tempfile::tempdir().expect("repo"),
        );
        let repo = dir.path();
        git(repo, &["init", "-q"]);
        std::fs::write(
            repo.join("t.md"),
            "a
",
        )
        .expect("a file");
        git(repo, &["add", "."]);
        git(
            repo,
            &[
                "-c",
                "user.name=T",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-qm",
                "one",
            ],
        );
        let (desk, cx) = crate::desk::tests::start(cx, data.path());
        crate::desk::tests::open(&desk, &repo.join("t.md"), cx);
        let said =
            |cx: &mut gpui::VisualTestContext| desk.read_with(cx, |desk, _| desk.tracked.clone());
        assert_eq!(said(cx), Some(Tracked::Unchanged));
        let (id, view) = desk.read_with(cx, |desk, _| {
            let tab = desk.active_tab().expect("a tab");
            (tab.id, tab.view.clone())
        });
        view.update(cx, |view, cx| {
            view.adopt(
                "b
"
                .into(),
                cx,
            )
        });
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
        desk.update_in(cx, |desk, window, cx| desk.save(id, None, window, cx));
        cx.run_until_parked();
        assert_eq!(
            std::fs::read_to_string(repo.join("t.md")).expect("saved"),
            "b
"
        );
        assert_eq!(said(cx), Some(Tracked::Changed));
    }
}
