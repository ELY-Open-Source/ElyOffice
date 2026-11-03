//! The existing native file dialogs open and save R source.

use super::{protocol::refused, runner::read_limited};
use crate::{book::Book, dialog::Dialog};
use ely_office::{in_window, prelude::*};
use gpui::{Context, PathPromptOptions, TaskExt as _};
use std::{
    io,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

/// Numbers file requests; only an editor's latest lands.
static ASKED: AtomicU64 = AtomicU64::new(0);

impl Book {
    pub(super) fn r_file(&mut self, save: bool, cx: &mut Context<Self>) {
        let (tab, text, ask) = {
            let Some(Dialog::RMacro(editor)) = &mut self.dialog else {
                return;
            };
            if editor.running.is_some() {
                return;
            }
            let ask = ASKED.fetch_add(1, Ordering::Relaxed) + 1;
            editor.asked = Some(ask);
            (editor.tab, editor.code.read(cx).text().to_string(), ask)
        };
        let Some(folder) = self.tab(tab).and_then(ely_office::folder) else {
            return self.panel_failed("no documents folder", cx);
        };
        let chosen = if save {
            let picked = cx.prompt_for_new_path(&folder, Some("macro.R"));
            cx.background_executor().spawn(async move {
                picked.await.map_err(io::Error::other)?.map_err(io::Error::other)
            })
        } else {
            let picked = cx.prompt_for_paths(PathPromptOptions {
                files: true,
                directories: false,
                multiple: false,
                prompt: None,
            });
            cx.background_executor().spawn(async move {
                let paths = picked.await.map_err(io::Error::other)?.map_err(io::Error::other)?;
                Ok(paths.and_then(|mut paths| paths.pop()))
            })
        };
        cx.spawn(async move |this, cx| {
            let chosen = chosen.await;
            let content = text.clone();
            let result = cx
                .background_executor()
                .spawn(async move {
                    match chosen? {
                        Some(path) => file(path, save, content),
                        None => Ok(None),
                    }
                })
                .await;
            in_window(&this, cx, |book, _, cx| book.r_filed(ask, &text, result, cx))
        })
        .detach_and_log_err(cx);
    }

    /// Lands request `ask` only in the editor that made it.
    fn r_filed(&mut self, ask: u64, typed: &str, result: io::Result<Option<String>>, cx: &mut Context<Self>) {
        let Some(Dialog::RMacro(editor)) = &mut self.dialog else {
            return log::info!("table: R file result dropped: the editor closed");
        };
        if editor.asked != Some(ask) || editor.running.is_some() {
            return log::info!("table: R file result dropped: the editor moved on");
        }
        editor.asked = None;
        let note = match result {
            Ok(Some(text)) if editor.code.read(cx).text() == typed => {
                editor.code.update(cx, |code, cx| code.set_text(text, cx));
                None
            }
            Ok(Some(_)) => Some(Self::text(cx, "table.r.kept", &[]).to_string()),
            Ok(None) => None,
            Err(error) => Some(super::said(&error, cx)),
        };
        if let Some(note) = note {
            self.r_note(&format!("
{note}"), cx);
        }
        cx.notify();
    }
}

fn file(path: PathBuf, save: bool, text: String) -> io::Result<Option<String>> {
    if path.extension().is_none_or(|e| !e.eq_ignore_ascii_case("R")) {
        return Err(refused("table.r.not_r_file"));
    }
    if save {
        // Staged and checked: a failed save keeps the old file.
        ely_files::save_atomic(&path, text.as_bytes(), 0)?;
        log::info!("table: R source saved");
        Ok(None)
    } else {
        let text = read_limited(&path)?;
        log::info!("table: R source opened");
        Ok(Some(text))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{RunRMacro, protocol::{BYTES, refusal}},
        file,
    };
    use crate::{book::Book, dialog::Dialog, tests::{book, settle, update}};
    use ely_office::prelude::*;
    use gpui::{AppContext as _, TestAppContext};

    #[test]
    fn only_r_files_open_and_save() {
        let dir = tempfile::tempdir().expect("directory");
        let text = dir.path().join("macro.txt");
        let error = file(text.clone(), true, "x".into()).expect_err("not .R");
        assert_eq!(refusal(&error), Some("table.r.not_r_file"));
        assert!(!text.exists(), "nothing written");
        let source = dir.path().join("trim.r");
        assert!(file(source.clone(), true, "x <- 1".into()).expect("saved").is_none());
        assert_eq!(file(source, false, String::new()).expect("read").as_deref(), Some("x <- 1"));
        let big = dir.path().join("big.R");
        std::fs::write(&big, vec![b'#'; BYTES as usize + 1]).expect("big file");
        assert_eq!(refusal(&file(big, false, String::new()).expect_err("too big")), Some("table.r.too_big"));
    }

    /// Endless, yet its size reads as zero.
    #[cfg(unix)]
    #[test]
    fn a_source_growing_past_the_limit_is_refused() {
        let dir = tempfile::tempdir().expect("directory");
        let endless = dir.path().join("endless.R");
        std::os::unix::fs::symlink("/dev/zero", &endless).expect("link");
        let error = file(endless, false, String::new()).expect_err("unbounded");
        assert_eq!(refusal(&error), Some("table.r.too_big"));
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_save_keeps_the_old_source() {
        use std::os::unix::fs::PermissionsExt as _;
        if unsafe { libc::geteuid() } == 0 {
            return eprintln!("SKIP: root ignores the folder's mode");
        }
        let dir = tempfile::tempdir().expect("directory");
        let source = dir.path().join("macro.R");
        std::fs::write(&source, "old <- 1").expect("old source");
        let mode = |bits| std::fs::Permissions::from_mode(bits);
        std::fs::set_permissions(dir.path(), mode(0o555)).expect("locked");
        let saved = file(source.clone(), true, "new <- 2".into());
        std::fs::set_permissions(dir.path(), mode(0o755)).expect("unlocked");
        assert!(saved.is_err(), "nothing can be staged");
        assert_eq!(std::fs::read_to_string(&source).expect("source"), "old <- 1");
    }

    fn code(cx: &mut TestAppContext, book: &gpui::Entity<Book>) -> String {
        cx.update(|app| match &book.read(app).dialog {
            Some(Dialog::RMacro(editor)) => editor.code.read(app).text().to_string(),
            _ => panic!("the R editor is open"),
        })
    }

    fn ask(cx: &mut TestAppContext, book: &gpui::Entity<Book>, ask: u64) {
        update(cx, book, |book, _| {
            if let Some(Dialog::RMacro(editor)) = &mut book.dialog {
                editor.asked = Some(ask);
            }
        });
    }

    #[test]
    fn only_the_latest_open_lands_in_untouched_code() {
        let dir = tempfile::tempdir().expect("directory");
        let mut cx = TestAppContext::single();
        let Some((window, book)) = book(&mut cx, dir.path()) else {
            return eprintln!("SKIP: no LOK build");
        };
        cx.update(ely_gpui_component::init_for_tests);
        update(&mut cx, &book, |book, cx| book.new_blank(cx));
        settle(&cx, |app| book.read(app).active_tab().is_some());
        let opened = |cx: &mut TestAppContext| {
            cx.update_window(window.into(), |_, window, app| {
                book.update(app, |book, cx| book.on_r_macro(&RunRMacro, window, cx))
            })
            .expect("the window lives");
        };
        opened(&mut cx);
        let typed = code(&mut cx, &book);
        let land = |cx: &mut TestAppContext, at: u64, typed: &str, text: &str| {
            update(cx, &book, |book, cx| book.r_filed(at, typed, Ok(Some(text.into())), cx));
        };
        // Two opens; the first answers last.
        ask(&mut cx, &book, 1);
        ask(&mut cx, &book, 2);
        land(&mut cx, 2, &typed, "newer <- 2");
        land(&mut cx, 1, &typed, "older <- 1");
        assert_eq!(code(&mut cx, &book), "newer <- 2");
        // Typing after the chooser closed keeps the typing.
        ask(&mut cx, &book, 3);
        land(&mut cx, 3, &typed, "late <- 3");
        assert_eq!(code(&mut cx, &book), "newer <- 2");
        let kept = cx.update(|app| Book::text(app, "table.r.kept", &[]).to_string());
        let output = cx.update(|app| match &book.read(app).dialog {
            Some(Dialog::RMacro(editor)) => editor.output.clone(),
            _ => String::new(),
        });
        assert!(output.contains(&kept), "{output}");
        // A reopened editor took no request.
        ask(&mut cx, &book, 4);
        update(&mut cx, &book, |book, _| book.dialog = None);
        opened(&mut cx);
        land(&mut cx, 4, "newer <- 2", "closed <- 4");
        assert_eq!(code(&mut cx, &book), "newer <- 2");
    }
}
