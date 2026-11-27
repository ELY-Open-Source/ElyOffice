//! Windows: the pages shown before the system print dialog.

use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use ely_gpui_component::{
    documents::{DocPage, PrintPreview},
    overlays::Dialog as Card,
};
use gpui::{AnyElement, App, Context, Entity, IntoElement, ParentElement as _, Styled as _, Window, div};

use crate::{Dialog, Tabs, dialogs::Dialogs, text};

/// Draws the print copy's pages, then shows them.
pub(crate) fn show<H: Tabs>(pdf: PathBuf, cx: &mut Context<H>) {
    let folder = pdf.with_extension("pages");
    cx.spawn(async move |this, cx| {
        let drawn = ely_platform::print::preview_pages(&pdf, &folder).await;
        let shown = this.update(cx, |host, cx| {
            let pages = match drawn {
                Ok(sheets) => sheets,
                Err(error) => {
                    drop_copy(&pdf, &folder);
                    return host.print_failed(&error.to_string(), cx);
                }
            };
            if host.core().dialog.is_some() {
                drop_copy(&pdf, &folder);
                return host.print_failed("another dialog is open", cx);
            }
            let pages = pages
                .into_iter()
                .map(|sheet| DocPage {
                    source: sheet.picture.to_string_lossy().into_owned().into(),
                    width: sheet.size.0,
                    height: sheet.size.1,
                })
                .collect();
            host.core_mut().dialog = Some(Dialog::Print { pdf: pdf.clone(), folder: folder.clone(), pages: Rc::new(pages) });
            cx.notify();
        });
        if shown.is_err() {
            log::info!("office: the window closed before the print preview");
            drop_copy(&pdf, &folder);
        }
    })
    .detach();
}

/// The full-window preview; Print opens the system dialog.
pub(crate) fn card<H: Tabs + Dialogs>(
    this: Entity<H>,
    (pdf, folder, pages): (&Path, &Path, &Rc<Vec<DocPage>>),
    cx: &App,
) -> AnyElement {
    let cancel = {
        let (this, pdf, folder) = (this.clone(), pdf.to_path_buf(), folder.to_path_buf());
        move |window: &mut Window, cx: &mut App| {
            log::info!("office: the print preview was cancelled");
            drop_copy(&pdf, &folder);
            this.update(cx, |host, cx| host.dialog_done(window, cx));
        }
    };
    let print = {
        let (pdf, folder) = (pdf.to_path_buf(), folder.to_path_buf());
        move |window: &mut Window, cx: &mut App| {
            drop_pages(&folder);
            this.update(cx, |host, cx| {
                host.dialog_done(window, cx);
                crate::print_now(pdf.clone(), cx);
            });
        }
    };
    let cancel = Rc::new(cancel);
    let closed = {
        let cancel = cancel.clone();
        move |window: &mut Window, cx: &mut App| cancel(window, cx)
    };
    Card::new("office-print-preview", text(cx, "office.print.preview", &[]), closed)
        .fullscreen()
        .child(div().size_full().child(PrintPreview::laid_out(
            "office-print",
            pages.clone(),
            print,
            move |window, cx| cancel(window, cx),
        )))
        .into_any_element()
}

/// Removes the print copy and its pictures.
fn drop_copy(pdf: &Path, folder: &Path) {
    // The copy may hold what a password guarded.
    if let Err(error) = std::fs::remove_file(pdf) {
        log::error!("office: the print copy {} stays: {error}", pdf.display());
    }
    drop_pages(folder);
}

fn drop_pages(folder: &Path) {
    match std::fs::remove_dir_all(folder) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            log::error!("office: the print pictures in {} stay: {error}", folder.display())
        }
        _ => {}
    }
}
