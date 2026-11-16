//! Two windows of one app share one record (W.1a).

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use ely_office::{After, Closing as _, Core, Dialog, Dialogs as _, Host, Kind, Opening as _, View};
use gpui::{AnyWindowHandle, AppContext as _, Context, Entity, IntoElement, Render, TestAppContext, Window};

use crate::engine::Mark;

/// A window of documents and nothing else.
struct Pane {
    core: Core<Pane>,
}

impl Host for Pane {
    type Extra = ();
    type Engine = Mark;
    const KIND: Kind = Kind { app: ely_platform::AppId::Mark, kept: &["md"], save_as: &["md"], macros: None };

    fn core(&self) -> &Core<Self> {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core<Self> {
        &mut self.core
    }

    fn observed(&mut self, _: u64, _: &Entity<View<Self>>, _: &mut Context<Self>) {}
}

impl Render for Pane {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui::div()
    }
}

/// The first window starts the record; the second joins it.
fn pane(work: Option<&Path>, cx: &mut TestAppContext) -> (Entity<Pane>, AnyWindowHandle) {
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |_, cx| {
            cx.new(|cx| {
                let core = match work {
                    Some(work) => Core::new(Arc::new(Mutex::new(())), None, work.to_path_buf(), cx),
                    None => Core::joined(cx).expect("the first window's record"),
                };
                Pane { core }
            })
        })
        .expect("a window")
    });
    let pane = window.root(cx).expect("the pane");
    window.update(cx, |pane, window, cx| pane.own_window(window, cx)).expect("owns its window");
    (pane, window.into())
}

fn open(pane: &Entity<Pane>, path: &Path, cx: &mut TestAppContext) {
    pane.update(cx, |pane, cx| pane.open_file(path.to_path_buf(), cx));
    cx.run_until_parked();
}

fn tabs(pane: &Entity<Pane>, cx: &mut TestAppContext) -> Vec<u64> {
    pane.read_with(cx, |pane, _| pane.core.tabs.iter().map(|tab| tab.id).collect())
}

fn asks(pane: &Entity<Pane>, cx: &mut TestAppContext) -> bool {
    pane.read_with(cx, |pane, _| matches!(pane.core.dialog, Some(Dialog::Unsaved { after: After::Quit, .. })))
}

fn start(cx: &mut TestAppContext) -> (tempfile::TempDir, [Entity<Pane>; 2], [AnyWindowHandle; 2]) {
    cx.update(ely_gpui_component::init_for_tests);
    cx.update(|cx| {
        ely_ui::settings::install_defaults(cx);
        ely_ui::i18n::install_as("en-US", cx);
    });
    let dir = tempfile::tempdir().expect("a folder");
    let work = dir.path().join("work");
    std::fs::create_dir(&work).expect("the work root");
    let (a, first) = pane(Some(&work), cx);
    let (b, second) = pane(None, cx);
    (dir, [a, b], [first, second])
}

#[gpui::test]
fn two_windows_share_ids_and_the_file_record(cx: &mut TestAppContext) {
    let (dir, [a, b], _) = start(cx);
    let (one, two) = (dir.path().join("one.md"), dir.path().join("two.md"));
    std::fs::write(&one, "One\n").expect("one");
    std::fs::write(&two, "Two\n").expect("two");
    open(&a, &one, cx);
    open(&b, &two, cx);
    let (in_a, in_b) = (tabs(&a, cx), tabs(&b, cx));
    assert_eq!((in_a.len(), in_b.len()), (1, 1));
    assert_ne!(in_a, in_b, "tab ids are the app's");
    let works = [&a, &b].map(|pane| pane.read_with(cx, |pane, _| pane.core.tabs[0].work.clone()));
    assert_ne!(works[0], works[1], "each tab has its own folder");
    // Open in the other window: that tab is picked.
    open(&b, &one, cx);
    assert_eq!(tabs(&b, cx), in_b, "no second copy of one file");
    assert_eq!(a.read_with(cx, |pane, _| pane.core.active()), Some(in_a[0]));
    // Closed there, it opens here.
    a.update(cx, |pane, cx| pane.close_tab(in_a[0], cx));
    open(&b, &one, cx);
    assert_eq!(tabs(&b, cx).len(), 2);
}

#[gpui::test]
fn quit_asks_each_window_and_a_cancel_stops_it(cx: &mut TestAppContext) {
    let (dir, [a, b], [first, second]) = start(cx);
    let (one, two) = (dir.path().join("one.md"), dir.path().join("two.md"));
    std::fs::write(&one, "One\n").expect("one");
    std::fs::write(&two, "Two\n").expect("two");
    open(&a, &one, cx);
    open(&b, &two, cx);
    for pane in [&a, &b] {
        let view = pane.read_with(cx, |pane, _| pane.core.tabs[0].view.clone());
        view.update(cx, |view, cx| view.adopt("Changed\n".into(), cx));
    }
    cx.run_until_parked();
    cx.update(ely_ui::commands::quit);
    cx.run_until_parked();
    assert!(asks(&a, cx) && !asks(&b, cx), "the first window asks first");
    first
        .update(cx, |_, window, cx| a.update(cx, |pane, cx| {
            pane.close_stopped(cx);
            pane.dialog_done(window, cx);
        }))
        .expect("cancels");
    cx.run_until_parked();
    assert!(!asks(&b, cx), "a cancel stops the whole quit");
    assert_eq!((tabs(&a, cx).len(), tabs(&b, cx).len()), (1, 1));
    cx.update(ely_ui::commands::quit);
    cx.run_until_parked();
    let id = tabs(&a, cx)[0];
    first
        .update(cx, |_, window, cx| a.update(cx, |pane, cx| {
            pane.close_tab(id, cx);
            pane.proceed(After::Quit, window, cx);
        }))
        .expect("discards");
    cx.run_until_parked();
    assert!(tabs(&a, cx).is_empty() && asks(&b, cx), "the next window asks");
    let id = tabs(&b, cx)[0];
    second
        .update(cx, |_, window, cx| b.update(cx, |pane, cx| {
            pane.close_tab(id, cx);
            pane.proceed(After::Quit, window, cx);
        }))
        .expect("discards");
    cx.run_until_parked();
    assert!(tabs(&b, cx).is_empty());
}
