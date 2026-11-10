//! Files dropped on a window: opened here or in their Ely app.

use std::{
    io,
    path::{Path, PathBuf},
};

use ely_platform::AppId;
use gpui::{App, AppContext as _};

use crate::Root;

/// The Ely app, other than `app`, that opens `path`'s type.
fn elsewhere(app: AppId, path: &Path) -> Option<AppId> {
    let ext = path.extension()?.to_str()?;
    ely_platform::apps::for_extension(ext).filter(|owner| *owner != app)
}

/// Opens each dropped file here, or in its type's Ely app.
pub fn open_dropped(app: AppId, paths: &[PathBuf], cx: &mut App) {
    for path in paths {
        match elsewhere(app, path) {
            None => {
                log::info!("drop: a file opens in {}", app.name());
                if let Err(error) = crate::opening::queue(path.clone(), cx) {
                    report(error, cx);
                }
            }
            Some(owner) => {
                log::info!("drop: a file goes to {}", owner.name());
                let path = path.clone();
                cx.spawn(async move |cx| {
                    let opened = cx.background_spawn(async move { ely_platform::apps::open_file(&path) }).await;
                    if let Err(error) = opened {
                        cx.update(|cx| report(error, cx));
                    }
                })
                .detach();
            }
        }
    }
}

/// Logs the failure; the main window, if open, shows it.
fn report(error: io::Error, cx: &mut App) {
    log::error!("drop: a file did not open: {}", ely_ipc::Worded::logged(&error));
    let Some(main) = crate::tray::main(cx) else {
        return;
    };
    // The drop may arrive inside that window's own update.
    cx.defer(move |cx| {
        let shown = main.update(cx, |view, _, cx| {
            if let Ok(root) = view.downcast::<Root>() {
                root.update(cx, |root, cx| root.open_failed(&error, cx));
            }
        });
        if shown.is_err() {
            log::warn!("drop: the main window closed before the failure showed");
        }
    });
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use ely_platform::AppId;
    use gpui::{ExternalPaths, FileDropEvent, IntoElement, Render, TestAppContext, VisualTestContext, div, point, px};

    use super::elsewhere;

    #[test]
    fn a_type_names_its_other_app_and_no_other() {
        assert_eq!(elsewhere(AppId::Doc, Path::new("/a/book.xlsx")), Some(AppId::Table));
        assert_eq!(elsewhere(AppId::Doc, Path::new("/a/DECK.PPTX")), Some(AppId::Show));
        assert_eq!(elsewhere(AppId::Doc, Path::new("/a/letter.docx")), None, "its own type");
        assert_eq!(elsewhere(AppId::Doc, Path::new("/a/photo.png")), None, "no Ely app's type");
        assert_eq!(elsewhere(AppId::Tree, Path::new("/a/folder")), None, "no extension");
        assert_eq!(elsewhere(AppId::Pdf, Path::new("/a/notes.md")), Some(AppId::Mark));
    }

    struct Page;

    impl Render for Page {
        fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            div()
        }
    }

    #[gpui::test]
    fn files_dropped_on_the_frame_reach_the_app_in_order(cx: &mut TestAppContext) {
        cx.update(|cx| {
            ely_gpui_component::init_for_tests(cx);
            crate::settings::install_defaults(cx);
            crate::i18n::install_as("en-US", cx);
            crate::opening::install_no_launch_files(cx);
        });
        let window = cx
            .update(|cx| {
                cx.open_window(Default::default(), |window, cx| {
                    let page = gpui::AppContext::new(cx, |_| Page);
                    crate::tool_root(AppId::Doc, page.into(), Vec::new(), window, cx)
                })
            })
            .expect("a window");
        let cx = &mut VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();
        let at = point(px(200.0), px(200.0));
        // Its own type, a type no Ely app has, a folder.
        let dropped = [PathBuf::from("/a/letter.docx"), PathBuf::from("/a/photo.png"), PathBuf::from("/a/folder")];
        cx.simulate_event(FileDropEvent::Entered { position: at, paths: ExternalPaths(dropped.to_vec().into()) });
        cx.simulate_event(FileDropEvent::Submit { position: at });
        cx.run_until_parked();
        assert_eq!(cx.update(|_, cx| crate::launch_files(cx)), dropped, "each goes to the app's own opening");
    }
}
