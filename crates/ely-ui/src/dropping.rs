//! Dropped files open here or in their Ely app.

use std::{
    io,
    path::{Path, PathBuf},
};

use ely_platform::AppId;
use gpui::{AnyWindowHandle, App, AppContext as _};

use crate::Root;

/// The Ely app, other than `app`, that opens `path`'s type.
fn elsewhere(app: AppId, path: &Path) -> Option<AppId> {
    let ext = path.extension()?.to_str()?;
    ely_platform::apps::for_extension(ext).filter(|owner| *owner != app)
}

/// Opens each dropped file in `window` or in its app.
pub fn open_dropped(app: AppId, paths: &[PathBuf], window: Option<AnyWindowHandle>, cx: &mut App) {
    hand(app, (paths, window), ely_platform::apps::open_file, cx);
}

/// Another app's files go through `open`, in drop order.
fn hand(app: AppId, (paths, window): (&[PathBuf], Option<AnyWindowHandle>), open: impl Fn(&Path) -> io::Result<()> + Send + 'static, cx: &mut App) {
    let mut away = Vec::new();
    for path in paths {
        match elsewhere(app, path) {
            None => {
                log::info!("drop: a file opens in {}", app.name());
                if let Err(error) = crate::opening::queue(path.clone(), window, cx) {
                    report(error, window, cx);
                }
            }
            Some(owner) => {
                log::info!("drop: a file goes to {}", owner.name());
                away.push(path.clone());
            }
        }
    }
    if away.is_empty() {
        return;
    }
    // One task: each file waits for the one before.
    cx.spawn(async move |cx| {
        let failed: Vec<io::Error> = cx.background_spawn(async move { away.iter().filter_map(|path| open(path).err()).collect() }).await;
        cx.update(|cx| failed.into_iter().for_each(|error| report(error, window, cx)));
    })
    .detach();
}

/// Logs the failure; shows it in the drop's window.
fn report(error: io::Error, window: Option<AnyWindowHandle>, cx: &mut App) {
    log::error!("drop: a file did not open: {}", ely_ipc::Worded::logged(&error));
    let Some(main) = window.or_else(|| crate::tray::main(cx)) else {
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
    use std::{
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };

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

    #[gpui::test(iterations = 20)]
    fn files_for_another_app_go_in_drop_order(cx: &mut TestAppContext) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let dropped = [PathBuf::from("/a/first.xlsx"), PathBuf::from("/a/second.xlsx"), PathBuf::from("/a/third.pptx")];
        let record = {
            let seen = Arc::clone(&seen);
            move |path: &Path| {
                seen.lock().expect("the record").push(path.to_path_buf());
                Ok(())
            }
        };
        cx.update(|cx| super::hand(AppId::Doc, (&dropped, None), record, cx));
        cx.run_until_parked();
        assert_eq!(*seen.lock().expect("the record"), dropped, "each waits for the one before");
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
        // Another window is in front; the drop lands here.
        let front = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                let page = gpui::AppContext::new(cx, |_| Page);
                crate::tool_root(AppId::Doc, page.into(), Vec::new(), window, cx)
            })
        });
        let front: gpui::AnyWindowHandle = front.expect("a front window").into();
        cx.update(|cx| {
            crate::tray::set_main(AppId::Doc, window.into(), cx);
            crate::tray::set_main(AppId::Doc, front, cx);
        });
        let cx = &mut VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();
        let at = point(px(200.0), px(200.0));
        // Its own type, a foreign type, a folder.
        let dropped = [PathBuf::from("/a/letter.docx"), PathBuf::from("/a/photo.png"), PathBuf::from("/a/folder")];
        cx.simulate_event(FileDropEvent::Entered { position: at, paths: ExternalPaths(dropped.to_vec().into()) });
        cx.simulate_event(FileDropEvent::Submit { position: at });
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let record = std::rc::Rc::clone(&seen);
        cx.update(|_, cx| crate::on_open(cx, move |path, window, _| record.borrow_mut().push((path, window))));
        cx.run_until_parked();
        let into: gpui::AnyWindowHandle = window.into();
        let expected: Vec<_> = dropped.iter().cloned().map(|path| (path, Some(into))).collect();
        assert_eq!(*seen.borrow(), expected, "each opens in the window it fell on");
        assert_eq!(cx.update(|_, cx| crate::tray::main(cx)), Some(front), "the drop raised nothing");
    }
}
