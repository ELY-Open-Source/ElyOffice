//! Terminal, file manager, a chosen editor, ElyMark (TREE-14).

use std::{
    io,
    path::{Path, PathBuf},
    process::{Child, Command},
};

use ely_gpui_component::{
    buttons::{ButtonVariant, IconButton},
    primitives::IconName,
    theme::ControlSize,
};
use ely_platform::AppId;
use gpui::{AppContext as _, Context, PathPromptOptions};

use crate::tree::{Tree, text};

/// Read once; written whenever the editor changes.
pub struct Tools {
    file: PathBuf,
    pub editor: Option<PathBuf>,
    /// Why the file was unreadable; never overwritten.
    pub broken: Option<String>,
}

impl Tools {
    /// The kept editor; a broken file is said, never overwritten.
    pub fn load(file: PathBuf) -> Tools {
        match Tools::read(file.clone()) {
            Ok(tools) => tools,
            Err(error) => Tools {
                file,
                editor: None,
                broken: Some(error.to_string()),
            },
        }
    }

    /// The editor kept in `file`; none until chosen.
    fn read(file: PathBuf) -> io::Result<Tools> {
        let editor = match std::fs::read(&file) {
            Ok(bytes) => {
                let said = |what: String| io::Error::other(format!("{}: {what}", file.display()));
                let read: serde_json::Value =
                    serde_json::from_slice(&bytes).map_err(|error| said(error.to_string()))?;
                match read.get("editor") {
                    _ if !read.is_object() => return Err(said("not an object".into())),
                    None => None,
                    Some(serde_json::Value::String(editor)) => Some(PathBuf::from(editor)),
                    Some(other) => return Err(said(format!("editor is {other}"))),
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        Ok(Tools {
            file,
            editor,
            broken: None,
        })
    }

    fn keep(&mut self, editor: PathBuf) -> io::Result<()> {
        if let Some(why) = &self.broken {
            return Err(io::Error::other(why.clone()));
        }
        let bytes = serde_json::to_vec(&serde_json::json!({ "editor": editor }))
            .map_err(io::Error::other)?;
        if let Some(dir) = self.file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        ely_files::save_atomic(&self.file, &bytes, 0)?;
        self.editor = Some(editor);
        Ok(())
    }
}

/// A bundle opens through `open`, waited on.
fn launch(editor: &Path, file: &Path) -> io::Result<()> {
    if cfg!(target_os = "macos") && editor.extension().is_some_and(|ext| ext == "app") {
        let out = Command::new("open")
            .arg("-a")
            .arg(editor)
            .arg(file)
            .output()?;
        if !out.status.success() {
            let said = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(io::Error::other(format!("{}: {said}", editor.display())));
        }
        return Ok(());
    }
    Command::new(editor)
        .arg(file)
        .spawn()
        .map(drop)
        .map_err(|error| io::Error::other(format!("{}: {error}", editor.display())))
}

/// The reveal entry, named for this platform's file browser.
pub const REVEAL: &str = if cfg!(target_os = "macos") {
    "tree.tool.reveal"
} else {
    "tree.tool.reveal_folder"
};

#[cfg(target_os = "macos")]
pub(crate) fn terminal_at(dir: &Path) -> io::Result<Child> {
    Command::new("open")
        .args(["-a", "Terminal"])
        .arg(dir)
        .stderr(std::process::Stdio::piped())
        .spawn()
}

/// PowerShell keeps a UNC folder; cmd.exe cannot.
#[cfg(windows)]
pub(crate) fn terminal(dir: &Path) -> Command {
    let mut command = Command::new("powershell.exe");
    command.arg("-NoLogo").current_dir(dir);
    command
}

/// A new console; Windows hosts it in the default terminal.
#[cfg(windows)]
pub(crate) fn terminal_at(dir: &Path) -> io::Result<Child> {
    use std::os::windows::process::CommandExt as _;
    // CREATE_NEW_CONSOLE
    terminal(dir).creation_flags(0x10).spawn()
}

/// The freedesktop launcher, else Debian's.
#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn terminal_at(dir: &Path) -> io::Result<Child> {
    launcher(&["xdg-terminal-exec", "x-terminal-emulator"], dir)
}

/// The first launcher found; its errors are kept.
#[cfg(all(unix, not(target_os = "macos")))]
fn launcher(names: &[&str], dir: &Path) -> io::Result<Child> {
    for &name in names {
        let mut command = Command::new(name);
        command.current_dir(dir).stderr(std::process::Stdio::piped());
        match command.spawn() {
            Ok(child) => return Ok(child),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                log::info!("tree: no {name}")
            }
            Err(error) => return Err(io::Error::other(format!("{name}: {error}"))),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("no terminal launcher: {}", names.join(" or ")),
    ))
}

/// After this, an exit is the session's own.
#[cfg(unix)]
const STARTUP: std::time::Duration = std::time::Duration::from_secs(5);

/// A launcher that fails while starting says why.
#[cfg(unix)]
fn settled(child: Child, startup: std::time::Duration) -> io::Result<()> {
    let started = std::time::Instant::now();
    let out = child.wait_with_output()?;
    // Some terminals end with their shell's code.
    if out.status.success() || started.elapsed() >= startup {
        log::info!("tree: the terminal ended, {}", out.status);
        return Ok(());
    }
    let said = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(io::Error::other(format!("{}: {said}", out.status)))
}

impl Tree {
    fn root(&self) -> Option<PathBuf> {
        self.active().map(|opened| opened.repo.root.clone())
    }

    pub fn open_terminal(&mut self, cx: &mut Context<Self>) {
        let Some(root) = self.root() else { return };
        log::info!("tree: terminal at {}", root.display());
        let child = match terminal_at(&root) {
            Ok(child) => child,
            Err(error) => return self.failed("tree.error.tool", &error.to_string(), cx),
        };
        // A closed console's exit says nothing.
        #[cfg(windows)]
        drop(child);
        #[cfg(unix)]
        {
            let (done, result) = futures::channel::oneshot::channel();
            // Waits as long as the terminal lives.
            std::thread::spawn(move || done.send(settled(child, STARTUP)));
            cx.spawn(async move |this, cx| {
                if let Ok(Err(error)) = result.await {
                    log::warn!("tree: the terminal failed: {error}");
                    this.update(cx, |tree, cx| {
                        tree.failed("tree.error.tool", &error.to_string(), cx)
                    })
                    .ok();
                }
            })
            .detach();
        }
    }

    pub fn reveal(&mut self, cx: &mut Context<Self>) {
        let Some(root) = self.root() else { return };
        cx.reveal_path(&root);
    }

    /// Asks which editor; `then` opens a file in it.
    pub fn pick_editor(&mut self, then: Option<String>, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: None,
        });
        cx.spawn(async move |this, cx| {
            let picked = picked
                .await
                .map_err(|error| error.to_string())
                .and_then(|picked| picked.map_err(|error| error.to_string()));
            this.update(cx, |tree, cx| match picked {
                Ok(Some(mut paths)) if !paths.is_empty() => {
                    let editor = paths.remove(0);
                    if let Err(error) = tree.tools.keep(editor) {
                        return tree.failed("tree.error.tool", &error.to_string(), cx);
                    }
                    if let Some(path) = then {
                        tree.open_in_editor(&path, cx);
                    }
                }
                Ok(_) => log::info!("tree: the editor panel was cancelled"),
                Err(error) => tree.failed("tree.error.panel", &error, cx),
            })
            .ok();
        })
        .detach();
    }

    /// A repository file in the chosen editor; asks first.
    pub fn open_in_editor(&mut self, path: &str, cx: &mut Context<Self>) {
        let Some(root) = self.root() else { return };
        let Some(editor) = self.tools.editor.clone() else {
            return self.pick_editor(Some(path.to_string()), cx);
        };
        let file = root.join(path);
        log::info!("tree: {} opens a file", editor.display());
        // The launcher's own failure arrives after it starts.
        let task = cx.background_spawn(async move { launch(&editor, &file) });
        cx.spawn(async move |this, cx| {
            if let Err(error) = task.await {
                this.update(cx, |tree, cx| {
                    tree.failed("tree.error.tool", &error.to_string(), cx)
                })
                .ok();
            }
        })
        .detach();
    }

    pub fn open_in_mark(&mut self, path: &str, cx: &mut Context<Self>) {
        let Some(root) = self.root() else { return };
        if let Err(error) = ely_platform::apps::open(AppId::Mark, &[root.join(path)]) {
            self.failed("tree.error.tool", &error.to_string(), cx);
        }
    }
}

impl Tree {
    /// The chosen editor, and ElyMark for Markdown.
    pub fn tool_buttons(&self, path: &str, cx: &mut Context<Self>) -> Vec<IconButton> {
        let (edit, mark) = (cx.weak_entity(), cx.weak_entity());
        let (at, md) = (path.to_string(), path.to_string());
        let mut buttons = vec![
            IconButton::new("tree-open-editor", IconName::Pencil)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(text(cx, "tree.tool.in_editor", &[]))
                .on_click(move |_, _, cx| {
                    edit.update(cx, |tree, cx| tree.open_in_editor(&at, cx))
                        .ok();
                }),
        ];
        if markdown(path) {
            buttons.push(
                IconButton::new("tree-open-mark", IconName::FileText)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .tooltip(text(cx, "tree.tool.in_mark", &[]))
                    .on_click(move |_, _, cx| {
                        mark.update(cx, |tree, cx| tree.open_in_mark(&md, cx)).ok();
                    }),
            );
        }
        buttons
    }
}

/// Markdown opens in ElyMark too.
pub fn markdown(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_editor_is_kept_and_read_back() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let file = dir.path().join("data/tools.json");
        let mut tools = super::Tools::read(file.clone()).expect("none yet");
        assert_eq!(tools.editor, None);
        tools.keep("/Applications/Zed.app".into()).expect("keeps");
        let again = super::Tools::read(file.clone()).expect("reads");
        assert_eq!(
            again.editor.as_deref(),
            Some(std::path::Path::new("/Applications/Zed.app"))
        );
        std::fs::write(&file, "not json").expect("writes");
        let mut broken = super::Tools::load(file.clone());
        assert!(broken.broken.is_some(), "a broken file is said");
        assert!(broken.keep("/x".into()).is_err());
        assert_eq!(
            std::fs::read(&file).expect("reads"),
            b"not json",
            "left alone"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_gone_editor_app_says_so() {
        let gone = std::path::Path::new("/nonexistent/DeletedEditor.app");
        let said = super::launch(gone, std::path::Path::new("/tmp")).expect_err("gone");
        assert!(said.to_string().contains("DeletedEditor.app"), "{said}");
    }

    /// The folder the terminal's shell starts in.
    #[cfg(windows)]
    fn shell_dir(dir: &std::path::Path) -> String {
        let out = super::terminal(dir)
            .args(["-NoProfile", "-Command", "(Get-Location).ProviderPath"])
            .output()
            .expect("powershell runs");
        assert!(out.status.success(), "{out:?}");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    #[cfg(windows)]
    #[test]
    fn the_terminal_starts_in_the_repository_even_on_a_share() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let local = dir.path().to_string_lossy().to_string();
        assert_eq!(shell_dir(dir.path()), local);
        let (drive, rest) = local.split_once(":\\").expect("a drive path");
        let share = format!("\\\\localhost\\{drive}$\\{rest}");
        if !std::path::Path::new(&share).is_dir() {
            return eprintln!("SKIP: no admin share at {share}");
        }
        assert_eq!(shell_dir(std::path::Path::new(&share)), share);
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn a_launcher_that_fails_while_starting_says_why() {
        use std::time::Duration;
        let dir = tempfile::tempdir().expect("a temp dir");
        // Written once in the tree: no file busy.
        let fixture = |name: &str| {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/tree").join(name);
            path.to_string_lossy().to_string()
        };
        let bad = fixture("launcher-fails");
        let started = super::launcher(&[&bad], dir.path()).expect("starts");
        let said = super::settled(started, Duration::from_secs(5)).expect_err("fails");
        assert!(said.to_string().contains("no display"), "{said}");
        let session = fixture("launcher-session");
        let started = super::launcher(&[&session], dir.path()).expect("starts");
        super::settled(started, Duration::from_millis(300)).expect("a session's own exit");
        let good = fixture("launcher-pwd");
        let started = super::launcher(&["ely-no-such-launcher", &good], dir.path()).expect("starts");
        super::settled(started, Duration::from_secs(5)).expect("ends well");
        let here = std::fs::read_to_string(dir.path().join("here")).expect("wrote");
        assert_eq!(std::path::Path::new(here.trim()), dir.path());
        let none = super::launcher(&["ely-no-such-launcher"], dir.path()).expect_err("none");
        assert_eq!(none.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn markdown_files_are_known_by_their_ending() {
        assert!(super::markdown("docs/READ.ME.md") && super::markdown("a.Markdown"));
        assert!(!super::markdown("notes.txt") && !super::markdown("md"));
    }
}
