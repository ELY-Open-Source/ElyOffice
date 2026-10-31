//! The existing native file dialogs open and save R source.

use crate::{book::Book, dialog::Dialog};
use ely_office::prelude::*;
use gpui::{Context, PathPromptOptions};
use std::{io, path::PathBuf};

impl Book {
    pub(super) fn r_file(&mut self, save: bool, cx: &mut Context<Self>) {
        let Some(Dialog::RMacro(editor)) = &self.dialog else {
            return;
        };
        if editor.running {
            return;
        }
        let code = editor.code.clone();
        let text = code.read(cx).text().to_string();
        let Some(folder) = self.tab(editor.tab).and_then(ely_office::folder) else {
            return;
        };
        let chosen = if save {
            let picked = cx.prompt_for_new_path(&folder, Some("macro.R"));
            cx.background_executor().spawn(async move {
                picked
                    .await
                    .map_err(|e| e.to_string())?
                    .map_err(|e| e.to_string())
            })
        } else {
            let picked = cx.prompt_for_paths(PathPromptOptions {
                files: true,
                directories: false,
                multiple: false,
                prompt: None,
            });
            cx.background_executor().spawn(async move {
                picked
                    .await
                    .map_err(|e| e.to_string())?
                    .map(|p| p.and_then(|mut p| p.pop()))
                    .map_err(|e| e.to_string())
            })
        };
        cx.spawn(async move |this, cx| {
            let chosen = chosen.await;
            let result = cx.background_executor().spawn(async move {
                let Some(path) = chosen.map_err(io::Error::other)? else { return Ok(None); };
                file(path, save, text)
            }).await;
            let _ = this.update(cx, |book, cx| {
                let same = matches!(&book.dialog, Some(Dialog::RMacro(editor)) if editor.code == code && !editor.running);
                if !same { return; }
                match result {
                    Ok(Some(text)) => code.update(cx, |code, cx| code.set_text(text, cx)),
                    Ok(None) => {},
                    Err(error) => book.r_output(&code, format!("\n{error}"), false, cx),
                }
                cx.notify();
            });
        }).detach();
    }
}

fn file(path: PathBuf, save: bool, text: String) -> io::Result<Option<String>> {
    if path
        .extension()
        .is_none_or(|e| !e.eq_ignore_ascii_case("R"))
    {
        return Err(io::Error::other("Choose a .R file"));
    }
    if save {
        std::fs::write(path, text)?;
        Ok(None)
    } else {
        if std::fs::metadata(&path)?.len() > super::protocol::BYTES {
            return Err(io::Error::other("R source exceeds 4 MiB"));
        }
        std::fs::read_to_string(path).map(Some)
    }
}
