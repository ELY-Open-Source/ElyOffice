use std::{cell::RefCell, path::Path, rc::Rc};

use ely_platform::AppId;
use gpui::{Entity, TestAppContext, VisualTestContext};

use super::{Desk, open_in};
use crate::view::Mode;

/// ElyMark over a fresh data folder, as `main` opens it.
pub(crate) fn start(
    cx: &mut TestAppContext,
    data: &Path,
) -> (Entity<Desk>, &'static mut VisualTestContext) {
    // The file watcher runs on a real thread.
    cx.executor().allow_parking();
    cx.update(ely_gpui_component::init_for_tests);
    cx.update(|cx| {
        ely_ui::settings::install_defaults(cx);
        ely_ui::i18n::install_as("en-US", cx);
        ely_ui::install_no_launch_files(cx);
    });
    let held = Rc::new(RefCell::new(None));
    let kept = held.clone();
    let data = data.to_path_buf();
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |window, cx| {
            let (root, desk) = open_in(AppId::Mark, data, window, cx);
            kept.replace(Some(desk));
            root
        })
        .expect("a window")
    });
    let cx = VisualTestContext::from_window(window.into(), cx).into_mut();
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let desk = held.take().expect("the desk");
    (desk, cx)
}

pub(crate) fn open(desk: &Entity<Desk>, path: &Path, cx: &mut VisualTestContext) {
    desk.update(cx, |desk, cx| desk.open_path(path.to_path_buf(), cx));
    cx.run_until_parked();
}

fn outline(desk: &Entity<Desk>, cx: &mut VisualTestContext) -> Vec<(usize, String, usize)> {
    desk.read_with(cx, |desk, cx| {
        let view = desk.active_view().expect("a tab");
        let outline = view.read(cx).outline(cx);
        outline
            .into_iter()
            .map(|heading| (heading.level, heading.title, heading.line))
            .collect()
    })
}

#[gpui::test]
fn a_file_opens_with_its_outline_and_joins_the_recent_list(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("a.md");
    std::fs::write(&path, "# One\n\nText.\n\n## Two\n\nMore.\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    assert_eq!(
        outline(&desk, cx),
        [(1, "One".into(), 0), (2, "Two".into(), 4)]
    );
    let view = desk.read_with(cx, |desk, _| desk.active_view().expect("a tab"));
    view.update_in(cx, |view, window, cx| view.reveal((4, 0), window, cx));
    cx.run_until_parked();
    assert_eq!(
        view.read_with(cx, |view, cx| view.place(cx).line),
        4,
        "the jump lands"
    );
    let real = dunce::canonicalize(&path).expect("real");
    assert!(desk.read_with(cx, |desk, _| desk.recent.contains(&real)));
    let kept = std::fs::read_to_string(data.path().join("recent.json")).expect("the list");
    assert!(kept.contains("a.md"), "{kept}");
    assert!(!kept.contains(r"\\\\?\\"), "plain paths only: {kept}");
}

#[gpui::test]
fn the_recent_list_clears_from_its_button(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("a.md");
    std::fs::write(&path, "Text.\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    assert!(desk.read_with(cx, |desk, _| !desk.recent.is_empty()));
    let button = cx.debug_bounds("mark-clear-recent").expect("the button");
    cx.simulate_click(button.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    assert!(desk.read_with(cx, |desk, _| desk.recent.is_empty()));
    let kept = std::fs::read_to_string(data.path().join("recent.json")).expect("the list");
    assert_eq!(kept, "[]");
    assert!(cx.debug_bounds("mark-clear-recent").is_none(), "nothing left to clear");
}

#[gpui::test]
fn one_recent_file_leaves_from_its_menu(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let (a, b) = (files.path().join("a.md"), files.path().join("b.md"));
    std::fs::write(&a, "A.\n").expect("a file");
    std::fs::write(&b, "B.\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &a, cx);
    open(&desk, &b, cx);
    let real = |path: &std::path::Path| dunce::canonicalize(path).expect("a real path");
    assert_eq!(desk.read_with(cx, |desk, _| desk.recent.clone()), [real(&b), real(&a)]);
    // The newest file is the first row.
    let remove_first = |cx: &mut gpui::VisualTestContext| {
        let list = cx.debug_bounds("mark-recent-list").expect("the recent list");
        let row = gpui::point(list.center().x, list.top() + gpui::px(12.0));
        cx.simulate_mouse_down(row, gpui::MouseButton::Right, gpui::Modifiers::none());
        cx.simulate_mouse_up(row, gpui::MouseButton::Right, gpui::Modifiers::none());
        cx.run_until_parked();
        let remove = cx.debug_bounds("menu-row-Remove from Recent").expect("the menu");
        cx.simulate_click(remove.center(), gpui::Modifiers::none());
        cx.run_until_parked();
    };
    remove_first(cx);
    assert_eq!(desk.read_with(cx, |desk, _| desk.recent.clone()), [real(&a)]);
    let kept = ely_files::recent::recent(&data.path().join("recent.json")).expect("the list");
    assert_eq!(kept, [real(&a)]);
    assert!(b.exists(), "the file stays on disk");
    assert_eq!(desk.read_with(cx, |desk, _| desk.core.tabs.len()), 2, "its tab stays");
    assert!(cx.debug_bounds("mark-clear-recent").is_some(), "clear all stays");
    // The list redrew: its first row is now a.
    remove_first(cx);
    assert!(desk.read_with(cx, |desk, _| desk.recent.is_empty()));
}

#[gpui::test]
fn the_outline_follows_typing(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("a.md");
    std::fs::write(&path, "Text.\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let view = desk.read_with(cx, |desk, _| desk.active_view().expect("a tab"));
    view.update_in(cx, |view, window, cx| view.switch(Mode::Source, window, cx));
    view.update(cx, |view, cx| view.adopt("# New\n\nText.\n".into(), cx));
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(1));
    cx.run_until_parked();
    assert_eq!(outline(&desk, cx), [(1, "New".into(), 0)]);
}

#[gpui::test]
fn two_files_are_two_tabs_each_with_its_outline(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let (a, b) = (files.path().join("a.md"), files.path().join("b.md"));
    std::fs::write(&a, "# A\n").expect("a");
    std::fs::write(&b, "# B\n").expect("b");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &a, cx);
    open(&desk, &b, cx);
    assert_eq!(desk.read_with(cx, |desk, _| desk.core.tabs.len()), 2);
    assert_eq!(outline(&desk, cx), [(1, "B".into(), 0)]);
    let first = desk.read_with(cx, |desk, _| desk.core.tabs[0].id);
    desk.update(cx, |desk, cx| {
        desk.core.active = Some(first);
        cx.notify();
    });
    assert_eq!(outline(&desk, cx), [(1, "A".into(), 0)]);
}

#[gpui::test]
fn a_folder_lists_its_markdown_files(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    std::fs::create_dir(files.path().join("sub")).expect("sub");
    std::fs::write(files.path().join("a.md"), "a").expect("a");
    std::fs::write(files.path().join("sub/b.markdown"), "b").expect("b");
    std::fs::write(files.path().join("c.txt"), "c").expect("c");
    let (desk, cx) = start(cx, data.path());
    open(&desk, files.path(), cx);
    let mut listed = desk.read_with(cx, |desk, _| {
        let folder = desk.folder.as_ref().expect("a folder");
        folder
            .files
            .iter()
            .map(|file| file.replace('\\', "/"))
            .collect::<Vec<_>>()
    });
    listed.sort();
    assert_eq!(listed, ["a.md", "sub/b.markdown"]);
}

#[gpui::test]
fn the_status_bar_names_the_caret_line_in_every_mode(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("a.md");
    std::fs::write(&path, "# A\n\nfirst\nsecond line\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let said = |cx: &mut VisualTestContext| {
        desk.read_with(cx, |desk, cx| desk.caret_said(cx).expect("a tab").to_string())
    };
    let view = desk.read_with(cx, |desk, _| desk.active_view().expect("a tab"));
    cx.update(|window, cx| view.update(cx, |view, cx| view.reveal((3, 4), window, cx)));
    cx.run_until_parked();
    assert_eq!(said(cx), "Ln 4, Col 5");
    for mode in [Mode::Source, Mode::Preview, Mode::Visual] {
        cx.update(|window, cx| desk.update(cx, |desk, cx| desk.switch(mode, window, cx)));
        cx.run_until_parked();
        assert_eq!(said(cx), "Ln 4, Col 5", "{mode:?}");
    }
}

#[gpui::test(iterations = 16)]
fn files_opened_together_keep_the_order_asked(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    // Larger files first, so the later loads finish sooner.
    let names = ["f0.md", "f1.md", "f2.md", "f3.md", "f4.md", "f5.md"];
    let paths: Vec<_> = names
        .iter()
        .enumerate()
        .map(|(n, name)| {
            let path = files.path().join(name);
            std::fs::write(&path, "Line\n\n".repeat(4000 >> n)).expect("a file");
            path
        })
        .collect();
    let (desk, cx) = start(cx, data.path());
    desk.update(cx, |desk, cx| {
        for path in &paths {
            desk.open_path(path.clone(), cx);
        }
    });
    cx.run_until_parked();
    let opened: Vec<String> = desk.read_with(cx, |desk, _| {
        desk.core.tabs.iter().map(|tab| tab.name.to_string()).collect()
    });
    assert_eq!(opened, names);
}

#[gpui::test]
fn a_closed_tab_frees_its_view_and_document(cx: &mut TestAppContext) {
    use ely_office::prelude::*;
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    let (a, b) = (files.path().join("a.md"), files.path().join("b.md"));
    std::fs::write(&a, "# A\n\nOne.\n").expect("a");
    std::fs::write(&b, "# B\n\nTwo.\n").expect("b");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &a, cx);
    for _ in 0..3 {
        open(&desk, &b, cx);
        let (id, view, document) = desk.read_with(cx, |desk, cx| {
            let tab = desk.core.tabs.last().expect("b's tab");
            (tab.id, tab.view.downgrade(), tab.view.read(cx).document().downgrade())
        });
        // Drawn once, as the window does.
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        desk.update(cx, |desk, cx| desk.close_tab(id, cx));
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        assert!(view.upgrade().is_none(), "the view outlives its tab");
        assert!(document.upgrade().is_none(), "the document outlives its tab");
        let remembered = desk.read_with(cx, |desk, _| desk.remembered.clone());
        assert!(!remembered.contains_key(&id), "the closed tab's path stays");
        assert_eq!(remembered.len(), 1, "only a.md is open: {remembered:?}");
    }
}
