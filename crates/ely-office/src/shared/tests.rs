//! Shared work outlives the window that began it.

use std::{
    io,
    path::Path,
    sync::{Arc, Mutex},
};

use gpui::{App, AppContext as _, Context, Entity, FocusHandle, TestAppContext, Window};

use crate::{Core, Engine, Host, Keeping, Kind, Load, Recovering as _, Stopping as _, View};

/// An engine that only counts its restarts.
struct Counter;

impl Engine for Counter {
    type Worker = usize;
    type Doc = ();
    type View = ();
    type Loaded = ();
    type Import = ();

    fn load(_: &mut usize, _: Load<()>) -> io::Result<()> {
        unreachable!("these tests load nothing")
    }

    fn open(_: &Arc<Mutex<usize>>, _: (), _: &mut Window, _: &mut App) -> io::Result<((), Entity<()>)> {
        unreachable!("these tests open nothing")
    }

    fn discard(_: &mut usize, _: ()) -> io::Result<()> {
        Ok(())
    }

    fn history(_: &mut usize, _: &()) -> io::Result<String> {
        Ok(String::new())
    }

    fn write(_: &mut usize, _: &(), _: &Path, _: &str, _: bool) -> io::Result<()> {
        unreachable!("these tests write nothing")
    }

    fn close(_: &mut usize, _: ()) -> io::Result<()> {
        Ok(())
    }

    fn retire(_: &Entity<()>, _: &mut App) -> ely_gpui_component::primitives::Retired {
        Default::default()
    }

    fn settled(_: &()) -> bool {
        true
    }

    fn edited(_: &()) -> u64 {
        0
    }

    fn edit_pending(_: &()) -> bool {
        false
    }

    fn focus(_: &()) -> FocusHandle {
        unreachable!("these tests focus nothing")
    }

    fn saved(_: &Entity<()>, _: &str, _: &mut App) {}

    fn restart(_: ely_platform::AppId, worker: &mut usize) -> io::Result<()> {
        *worker += 1;
        Ok(())
    }

    fn watch<H: Host<Engine = Self>>(
        _: &Entity<()>,
        _: &mut Context<H>,
        _: impl Fn(&mut H, &Entity<()>, bool, &mut Context<H>) + 'static,
        _: impl Fn(&mut H, &str, &mut Context<H>) + 'static,
    ) -> Vec<gpui::Subscription> {
        Vec::new()
    }
}

struct Pane {
    core: Core<Self>,
}

impl Host for Pane {
    type Extra = ();
    type Engine = Counter;
    const KIND: Kind = Kind { app: ely_platform::AppId::Mark, kept: &["md"], save_as: &["md"], macros: None };

    fn core(&self) -> &Core<Self> {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core<Self> {
        &mut self.core
    }

    fn observed(&mut self, _: u64, _: &Entity<View<Self>>, _: &mut Context<Self>) {}
}

/// Two windows over one record kept in `dir`.
fn panes(dir: &Path, cx: &mut TestAppContext) -> [Entity<Pane>; 2] {
    cx.update(ely_gpui_component::init_for_tests);
    cx.update(|cx| ely_ui::i18n::install_as("en-US", cx));
    let stop: crate::Stop = Arc::new(|| Ok(()));
    let first = cx.new(|cx| Pane { core: Core::new(Arc::new(Mutex::new(0)), Some(stop), dir.to_path_buf(), cx) });
    let second = cx.new(|cx| Pane { core: Core::joined(cx).expect("the first window's record") });
    let keeping = Keeping::in_folder(dir.to_path_buf()).expect("recovery");
    first.update(cx, |pane, _| pane.core.shared.get_mut().keeping = Some(keeping));
    [first, second]
}

fn index(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("recovery/index.json")).expect("the index")
}

#[gpui::test]
fn the_index_still_writes_after_its_window_closes(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a folder");
    let [first, second] = panes(dir.path(), cx);
    first.update(cx, |pane, cx| pane.keep_tab(0, ("one.md", None), false, cx));
    drop(first);
    cx.run_until_parked();
    second.update(cx, |pane, cx| pane.keep_tab(1, ("two.md", None), false, cx));
    cx.run_until_parked();
    let index = index(dir.path());
    assert!(index.contains("one.md") && index.contains("two.md"), "{index}");
}

#[gpui::test]
fn a_restart_ends_after_its_window_closes(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a folder");
    let [first, second] = panes(dir.path(), cx);
    first.update(cx, |pane, cx| pane.swap_engine(9, cx));
    drop(first);
    cx.run_until_parked();
    second.read_with(cx, |pane, _| {
        assert!(pane.core.shared.get().stopped.is_none(), "the restart ended");
        assert_eq!(*pane.core.worker.lock().expect("the worker"), 1, "one restart");
    });
}

#[gpui::test]
fn both_windows_open_share_the_index_and_the_restart(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a folder");
    let [first, second] = panes(dir.path(), cx);
    first.update(cx, |pane, cx| pane.keep_tab(0, ("one.md", None), false, cx));
    second.update(cx, |pane, cx| pane.keep_tab(1, ("two.md", None), false, cx));
    cx.run_until_parked();
    let index = index(dir.path());
    assert!(index.contains("one.md") && index.contains("two.md"), "{index}");
    first.update(cx, |pane, cx| pane.swap_engine(9, cx));
    second.update(cx, |pane, cx| pane.swap_engine(9, cx));
    cx.run_until_parked();
    second.read_with(cx, |pane, _| {
        assert!(pane.core.shared.get().stopped.is_none(), "the restart ended");
        assert_eq!(*pane.core.worker.lock().expect("the worker"), 1, "one restart, not two");
    });
}

#[gpui::test]
fn a_tab_writing_its_point_stays(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a folder");
    let [first, _] = panes(dir.path(), cx);
    first.update(cx, |pane, cx| {
        pane.keep_tab(0, ("one.md", None), false, cx);
        let view = cx.new(|_| ());
        let session = ely_files::Session::new_file();
        let (work, links) = (dir.path().to_path_buf(), Default::default());
        let tab = crate::Tab { id: 0, view, doc: (), session, registry: None, name: "one.md".into(), modified: false, risk: None, links, work, exporting: false, pausing: 0, reopen: false, extra: (), watching: Vec::new(), busy: Default::default() };
        pane.core.tabs.push(tab);
    });
    cx.run_until_parked();
    let writing = |pane: &Entity<Pane>, on: bool, cx: &mut TestAppContext| {
        pane.update(cx, |pane, _| pane.core.shared.get_mut().keeping.as_mut().expect("recovery").set_writing(0, on));
    };
    writing(&first, true, cx);
    let stays = first.update(cx, |pane, cx| crate::moving::Moving::move_out(pane, 0, cx).err());
    assert_eq!(stays, Some("office.move.saving"), "a point in flight keeps the tab");
    writing(&first, false, cx);
    let left = first.update(cx, |pane, cx| crate::moving::Moving::move_out(pane, 0, cx).is_ok());
    assert!(left, "once written, the tab may move");
}
