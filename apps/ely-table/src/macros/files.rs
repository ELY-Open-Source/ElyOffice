//! The existing native file dialogs open and save R source.

use super::protocol::{BYTES, refused};
use crate::{book::Book, dialog::Dialog};
use ely_office::{in_window, prelude::*};
use gpui::{Context, PathPromptOptions, TaskExt as _};
use std::{io, path::PathBuf};

impl Book {
    pub(super) fn r_file(&mut self, save: bool, cx: &mut Context<Self>) {
        let Some(Dialog::RMacro(editor)) = &self.dialog else {
            return;
        };
        if editor.running.is_some() {
            return;
        }
        let code = editor.code.clone();
        let text = code.read(cx).text().to_string();
        let Some(folder) = self.tab(editor.tab).and_then(ely_office::folder) else {
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
            let result = cx
                .background_executor()
                .spawn(async move {
                    match chosen? {
                        Some(path) => file(path, save, text),
                        None => Ok(None),
                    }
                })
                .await;
            in_window(&this, cx, |book, _, cx| {
                let same = matches!(&book.dialog, Some(Dialog::RMacro(editor)) if editor.code == code && editor.running.is_none());
                if !same {
                    return log::info!("table: R file result dropped: the editor moved on");
                }
                match result {
                    Ok(Some(text)) => code.update(cx, |code, cx| code.set_text(text, cx)),
                    Ok(None) => {}
                    Err(error) => book.r_note(&format!("\n{}", super::said(&error, cx)), cx),
                }
                cx.notify();
            })
        })
        .detach_and_log_err(cx);
    }
}

fn file(path: PathBuf, save: bool, text: String) -> io::Result<Option<String>> {
    if path.extension().is_none_or(|e| !e.eq_ignore_ascii_case("R")) {
        return Err(refused("table.r.not_r_file"));
    }
    if save {
        log::info!("table: R source saved");
        std::fs::write(path, text)?;
        Ok(None)
    } else {
        if std::fs::metadata(&path)?.len() > BYTES {
            return Err(refused("table.r.too_big"));
        }
        log::info!("table: R source opened");
        std::fs::read_to_string(path).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::{super::protocol::refusal, file};

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
        std::fs::write(&big, vec![b'#'; super::BYTES as usize + 1]).expect("big file");
        assert_eq!(refusal(&file(big, false, String::new()).expect_err("too big")), Some("table.r.too_big"));
    }
}
