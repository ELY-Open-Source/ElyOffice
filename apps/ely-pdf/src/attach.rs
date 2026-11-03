//! Attachments: listed beside the pages, saved, added, removed (PDF-07).

use std::path::PathBuf;

use ely_gpui_component::{
    feedback::EmptyState,
    lists::{ListItem, SelectableList},
    menus::{ContextMenu, Menu as Popup, MenuItem as PopupItem},
    primitives::{IconName, Severity},
};
use ely_office::{Closing as _, Saving as _, Tabs as _, in_window};
use gpui::{
    Action, AnyElement, App, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, TaskExt as _, div,
};

use crate::document::{RemoveAttachment, SaveAttachment};
use crate::view::{PdfView, Unread};
use crate::viewer::Viewer;

/// A size as people read it.
pub fn size(bytes: u64) -> String {
    match bytes {
        0..1024 => format!("{bytes} B"),
        1024..1_048_576 => format!("{:.1} KB", bytes as f64 / 1024.0),
        _ => format!("{:.1} MB", bytes as f64 / 1_048_576.0),
    }
}

impl PdfView {
    /// Reads the attachments once an edit.
    pub fn ask_files(&mut self, cx: &mut Context<Self>) {
        if self.files.is_some() || self.reading_files || self.unread.contains(&Unread::Files) {
            return;
        }
        self.reading_files = true;
        let (doc, version) = (self.doc, self.version);
        self.off(
            move |worker| worker.attachments(doc),
            move |view, read, cx| {
                view.reading_files = false;
                if view.version != version {
                    return cx.notify();
                }
                match read {
                    Ok(files) => view.files = Some(files),
                    Err(error) => {
                        view.unread.insert(Unread::Files);
                        view.fail("pdf.doing.attachments", &error, cx)
                    }
                }
                cx.notify();
            },
            cx,
        );
    }
}

/// The attachments of `view`; a right press saves or removes.
pub fn list(view: &Entity<PdfView>, cx: &mut App) -> AnyElement {
    view.update(cx, |view, cx| view.ask_files(cx));
    let read = view.read(cx);
    let Some(files) = read.files.as_ref() else {
        return div().into_any_element();
    };
    if files.is_empty() {
        return EmptyState::new("pdf-files-none", IconName::File, Viewer::text(cx, "pdf.files.none", &[])).into_any_element();
    }
    let total = files.len();
    let rows = files.iter().enumerate().fold(SelectableList::new("pdf-files"), |list, (at, file)| {
        let mut about = size(file.size);
        if !file.mime.is_empty() {
            about = format!("{about} · {}", file.mime);
        }
        if let Some(page) = file.page {
            about = format!("{about} · {}", Viewer::text(cx, "pdf.page.label", &[("n", &(page + 1).to_string())]));
        }
        let mut item = ListItem::new(("pdf-file", at), file.name.clone()).description(about).place(at + 1, total);
        if !file.note.is_empty() {
            item = item.detail(file.note.clone());
        }
        list.row(file.key.clone(), item)
    });
    let picked = view.clone();
    let rows = rows
        .selected(read.file_pick.clone())
        .label(Viewer::text(cx, "pdf.side.files", &[]))
        .on_change(move |keys, _, cx| {
            let name = keys.first().map(|key| key.to_string());
            picked.update(cx, |view, cx| {
                view.file_pick = name;
                cx.notify();
            })
        });
    let item = |key: &str, action: Box<dyn Action>| {
        PopupItem::new(Viewer::text(cx, key, &[])).on_click(move |window, cx| window.dispatch_action(action.boxed_clone(), cx))
    };
    let popup = Popup::new().item(item("pdf.file.save", Box::new(SaveAttachment))).item(item("pdf.file.remove", Box::new(RemoveAttachment)));
    let body = div().id("pdf-files-scroll").size_full().overflow_y_scroll().child(rows);
    ContextMenu::new("pdf-files-menu", popup).size_full().child(body).into_any_element()
}

impl Viewer {
    /// Asks for a file, then embeds it by name.
    pub fn attach_file(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.active_view() else {
            return self.no_document(cx);
        };
        self.pick(false, cx, move |_, path: PathBuf, cx| {
            let name = ely_office::files::file_name(&path);
            log::info!("pdf: attaches a file");
            view.update(cx, |view, cx| view.edit("pdf.doing.attach", move |worker, doc| worker.attach(doc, &name, &path), cx));
        });
    }

    pub fn remove_attachment(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.active_view() else {
            return self.no_document(cx);
        };
        let Some(key) = view.read(cx).file_pick.clone() else {
            return;
        };
        view.update(cx, |view, cx| {
            view.file_pick = None;
            view.edit("pdf.doing.remove_attachment", move |worker, doc| worker.detach(doc, &key), cx)
        });
    }

    /// Asks where, then writes the picked attachment there.
    pub fn save_attachment(&mut self, cx: &mut Context<Self>) {
        let (Some(tab), Some(view)) = (self.active_tab(), self.active_view()) else {
            return self.no_document(cx);
        };
        let read = view.read(cx);
        let picked = read.file_pick.as_ref().and_then(|key| read.files.as_ref()?.iter().find(|file| &file.key == key));
        let Some((key, name)) = picked.map(|file| (file.key.clone(), file.name.clone())) else {
            return;
        };
        let Some(dir) = ely_office::folder(tab) else {
            return self.panel_failed(&ely_ui::i18n::text(cx, "error.no_documents", &[]), cx);
        };
        let id = tab.id;
        let chosen = cx.prompt_for_new_path(&dir, Some(&name));
        cx.spawn(async move |this, cx| {
            let chosen = match chosen.await {
                Ok(Ok(path)) => Ok(path),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            in_window(&this, cx, |viewer, _, cx| match chosen {
                Ok(Some(target)) => viewer.write_attachment(id, key, target, cx),
                Ok(None) => log::info!("pdf: the save panel was cancelled"),
                Err(error) => viewer.panel_failed(&error, cx),
            })
        })
        .detach_and_log_err(cx);
    }

    pub(crate) fn write_attachment(&mut self, id: u64, key: String, target: PathBuf, cx: &mut Context<Self>) {
        let Some(tab) = self.tab(id) else {
            return;
        };
        let (shown, opened) = (ely_office::files::file_name(&target), target.clone());
        match self.holder(&target) {
            Ok(None) => {}
            Ok(Some(_)) => return self.toast(Self::text(cx, "pdf.extract.taken", &[("name", &shown)]), Severity::Danger, cx),
            Err(error) => return self.failed(&error, cx),
        }
        let (open, revision) = (tab.session.path().map(PathBuf::from), tab.session.revision());
        self.read_in(
            id,
            move |worker, doc| {
                let bytes = worker.get()?.attachment(*doc, &key)?;
                ely_files::export(&target, open.as_deref(), &bytes, revision).map(|_| ())
            },
            move |viewer, done, _, cx| match done {
                Ok(()) => viewer.toast_output(Self::text(cx, "pdf.extract.done", &[("name", &shown)]), opened.clone(), cx),
                Err(error) => viewer.failed(&error, cx),
            },
            cx,
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sizes_read_in_their_unit() {
        assert_eq!(super::size(33), "33 B");
        assert_eq!(super::size(2048), "2.0 KB");
        assert_eq!(super::size(3 * 1_048_576), "3.0 MB");
    }
}
