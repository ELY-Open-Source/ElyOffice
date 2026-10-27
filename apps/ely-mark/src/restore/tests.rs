use crate::view::Mode;

#[test]
fn a_place_without_a_column_or_top_reads_zero() {
    let kept: super::Kept = serde_json::from_str(
        r#"{"path":null,"name":"a.md","mode":"source","place":{"line":3,"scroll":0},"slot":"s"}"#,
    )
    .expect("an older place reads");
    let place = kept.place.expect("a place");
    assert_eq!((place.line, place.column, place.top), (3, 0, 0));
}

#[test]
fn each_untitled_copy_goes_to_the_tab_made_for_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let kept = |name: &str, mode: Mode, slot: &str| super::Kept {
        path: None,
        name: name.into(),
        mode,
        place: Some(super::Place {
            line: slot.len() as u64,
            column: 1,
            top: 0,
        }),
        slot: slot.into(),
    };
    let tabs = vec![
        kept("Untitled1", Mode::Visual, "a"),
        kept("Untitled2", Mode::Source, "bb"),
    ];
    for (slot, text) in [("a", "first"), ("bb", "second")] {
        let mut points = super::Recovery::new(super::recovery(dir.path(), slot));
        points
            .write_once(text.as_bytes(), None, std::time::SystemTime::now())
            .expect("a point");
    }
    let saved = serde_json::to_vec(&super::Saved { tabs: tabs.clone() }).expect("a list");
    std::fs::write(dir.path().join("session.json"), saved).expect("the list");
    // A pid no process has: the last run crashed.
    std::fs::write(dir.path().join("session.lock"), "open 4294967295").expect("a lock");
    let (mut keeping, ..) = super::Keeping::start(dir.path()).expect("a session");
    let made: Vec<_> = keeping.untitled();
    assert_eq!(
        made,
        [(0, "Untitled1".to_string()), (1, "Untitled2".to_string())]
    );
    for (at, _) in made {
        keeping.made(at, 10 + at as u64);
    }
    assert!(keeping.take(None, 12).is_none(), "a new tab takes nothing");
    // The second tab shows first: each still takes its own.
    for (id, want, text) in [(11, &tabs[1], "second"), (10, &tabs[0], "first")] {
        let got = keeping.take(None, id).expect("its copy");
        assert_eq!(
            (&got.kept, got.text.map(|(text, _)| text)),
            (want, Some(text.to_string()))
        );
    }
}

#[test]
fn an_unasked_end_keeps_the_lock_and_points_past_release() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let point = dir.path().join("recovery/s/point");
    let (lock, slot) = (
        dir.path().join("session.lock"),
        point.parent().expect("a slot"),
    );
    let (mut held, ..) = super::Keeping::start(dir.path()).expect("a session");
    std::fs::create_dir_all(slot).expect("a slot");
    std::fs::write(&point, "unsaved").expect("a point");
    held.hold();
    held.close();
    assert!(
        lock.exists() && point.exists(),
        "release after quit wiped them"
    );
    drop(held);
    let (mut clean, ..) = super::Keeping::start(dir.path()).expect("a session");
    std::fs::create_dir_all(slot).expect("a slot");
    std::fs::write(&point, "unsaved").expect("a point");
    clean.close();
    assert!(!lock.exists() && !point.exists(), "a clean end clears");
}

mod window {
    use gpui::{Entity, TestAppContext, VisualTestContext};

    use crate::desk::{
        Desk,
        tests::{open, start},
    };
    use crate::view::{MarkView, Mode};

    /// Shows the tab named `name`; its view.
    fn show(desk: &Entity<Desk>, name: &str, cx: &mut VisualTestContext) -> Entity<MarkView> {
        let view = desk.update(cx, |desk, cx| {
            let tab = desk.core.tabs.iter().find(|tab| tab.name.as_ref() == name);
            let tab = tab.unwrap_or_else(|| panic!("no tab {name}"));
            let (id, view) = (tab.id, tab.view.clone());
            desk.core.active = Some(id);
            cx.notify();
            view
        });
        cx.run_until_parked();
        view
    }

    fn names(desk: &Entity<Desk>, cx: &mut VisualTestContext) -> Vec<String> {
        desk.read_with(cx, |desk, _| {
            desk.core
                .tabs
                .iter()
                .map(|tab| tab.name.to_string())
                .collect()
        })
    }

    #[gpui::test]
    fn a_new_launch_brings_back_tabs_modes_places_and_look(
        first: &mut TestAppContext,
        second: &mut TestAppContext,
    ) {
        let (data, files) = (
            tempfile::tempdir().expect("data"),
            tempfile::tempdir().expect("files"),
        );
        let long: String = (0..80).map(|n| format!("Line {n}\n\n")).collect();
        let (a, b) = (files.path().join("a.md"), files.path().join("b.md"));
        std::fs::write(&a, &long).expect("a");
        std::fs::write(&b, "# B\n").expect("b");
        let (desk, cx) = start(first, data.path());
        open(&desk, &a, cx);
        open(&desk, &b, cx);
        let view = show(&desk, "a.md", cx);
        view.update_in(cx, |view, window, cx| {
            view.switch(Mode::Source, window, cx);
            view.document().update(cx, |document, cx| {
                document.set_caret(40, 2, cx);
                document.set_top_line(30, window, cx);
            });
        });
        cx.run_until_parked();
        desk.update(cx, |desk, cx| {
            desk.relook(|look| look.focus = true, cx);
            desk.leave(cx);
            desk.keeping.close();
        });
        cx.run_until_parked();

        let (desk, cx) = start(second, data.path());
        assert_eq!(names(&desk, cx), ["a.md", "b.md"]);
        let view = show(&desk, "a.md", cx);
        let (mode, place) = view.read_with(cx, |view, cx| (view.mode, view.place(cx)));
        assert_eq!(mode, Mode::Source);
        assert_eq!((place.line, place.column, place.top), (40, 2, 30));
        assert!(
            desk.read_with(cx, |desk, _| desk.look.focus),
            "the look came back"
        );
    }

    #[gpui::test]
    fn a_crash_brings_unsaved_text_back_as_a_copy(
        first: &mut TestAppContext,
        second: &mut TestAppContext,
    ) {
        let (data, files) = (
            tempfile::tempdir().expect("data"),
            tempfile::tempdir().expect("files"),
        );
        let a = files.path().join("a.md");
        std::fs::write(&a, "Text.\n").expect("a");
        let (desk, cx) = start(first, data.path());
        open(&desk, &a, cx);
        let view = show(&desk, "a.md", cx);
        view.update(cx, |view, cx| view.adopt("Changed.\n".into(), cx));
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
        desk.update(cx, |desk, cx| desk.keep(cx));
        cx.run_until_parked();
        // A pid no process has: this run crashed.
        std::fs::write(data.path().join("session.lock"), "open 4294967295").expect("a lock");

        let (desk, cx) = start(second, data.path());
        assert_eq!(names(&desk, cx).len(), 2, "{:?}", names(&desk, cx));
        let copy = desk.read_with(cx, |desk, _| {
            let tab = desk
                .core
                .tabs
                .iter()
                .find(|tab| tab.session.path().is_none());
            tab.expect("an untitled copy").name.to_string()
        });
        show(&desk, &copy, cx);
        let (text, dirty) = desk.read_with(cx, |desk, cx| {
            let tab = desk
                .core
                .tabs
                .iter()
                .find(|tab| tab.name.as_ref() == copy)
                .expect("the copy");
            (desk.text_of(tab.id).expect("text"), tab.dirty(cx))
        });
        assert_eq!(text, "Changed.\n");
        assert!(dirty, "the copy is unsaved");
        assert_eq!(
            std::fs::read_to_string(&a).expect("reads"),
            "Text.\n",
            "the file stays"
        );
    }

    #[test]
    fn a_kept_mode_no_build_knows_does_not_read() {
        let read = serde_json::from_str::<super::super::Kept>(
            r#"{"path":null,"name":"a.md","mode":"split","place":null,"slot":"s"}"#,
        );
        assert!(read.is_err(), "an unknown mode is no default");
    }

    /// The window's toasts that hold `words`.
    fn toasted(desk: &Entity<Desk>, words: &str, cx: &mut VisualTestContext) -> usize {
        let root = desk.read_with(cx, |desk, _| desk.core.root.upgrade().expect("the window"));
        let toasts = root.read_with(cx, |root, cx| root.toasts(cx));
        toasts
            .iter()
            .filter(|(said, _)| said.contains(words))
            .count()
    }

    #[gpui::test]
    fn a_failed_recovery_write_shows_once_with_the_last_kept(cx: &mut TestAppContext) {
        let (data, files) = (
            tempfile::tempdir().expect("data"),
            tempfile::tempdir().expect("files"),
        );
        let a = files.path().join("a.md");
        std::fs::write(
            &a, "Text.
",
        )
        .expect("a");
        let (desk, cx) = start(cx, data.path());
        open(&desk, &a, cx);
        let view = show(&desk, "a.md", cx);
        for (typed, broken) in [
            (
                "One.
", false,
            ),
            (
                "Two.
", true,
            ),
            (
                "Three.
", true,
            ),
        ] {
            if broken && data.path().join("recovery").is_dir() {
                std::fs::remove_dir_all(data.path().join("recovery")).expect("removes");
                std::fs::write(data.path().join("recovery"), "not a folder").expect("blocks");
            }
            view.update(cx, |view, cx| view.adopt(typed.into(), cx));
            cx.executor()
                .advance_clock(std::time::Duration::from_secs(1));
            cx.run_until_parked();
            desk.update(cx, |desk, cx| desk.keep(cx));
            cx.run_until_parked();
        }
        assert_eq!(toasted(&desk, "was not kept for recovery", cx), 1, "once");
        let never = toasted(&desk, "none yet", cx);
        assert_eq!(never, 0, "the first point landed, so its time shows");
    }

    #[gpui::test]
    fn a_failed_session_write_shows_once(cx: &mut TestAppContext) {
        let (data, files) = (
            tempfile::tempdir().expect("data"),
            tempfile::tempdir().expect("files"),
        );
        let a = files.path().join("a.md");
        std::fs::write(
            &a, "Text.
",
        )
        .expect("a");
        let (desk, cx) = start(cx, data.path());
        open(&desk, &a, cx);
        let session = data.path().join("session.json");
        if session.exists() {
            std::fs::remove_file(&session).expect("removes");
        }
        std::fs::create_dir(&session).expect("blocks");
        for line in [1, 2] {
            let view = show(&desk, "a.md", cx);
            view.update(cx, |view, cx| {
                view.document()
                    .update(cx, |document, cx| document.set_caret(line, 0, cx))
            });
            desk.update(cx, |desk, cx| desk.keep(cx));
            cx.run_until_parked();
        }
        assert_eq!(
            toasted(&desk, "were not saved for the next launch", cx),
            1,
            "once"
        );
    }

    /// Types `typed` into the shown tab, then keeps.
    fn type_and_keep(
        desk: &Entity<Desk>,
        view: &Entity<MarkView>,
        typed: &str,
        cx: &mut VisualTestContext,
    ) {
        view.update(cx, |view, cx| view.adopt(typed.into(), cx));
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
        desk.update(cx, |desk, cx| desk.keep(cx));
        cx.run_until_parked();
    }

    #[gpui::test]
    fn a_recovered_tab_names_its_point_when_the_next_fails(
        first: &mut TestAppContext,
        second: &mut TestAppContext,
    ) {
        let (data, files) = (
            tempfile::tempdir().expect("data"),
            tempfile::tempdir().expect("files"),
        );
        let a = files.path().join("a.md");
        std::fs::write(
            &a, "Text.
",
        )
        .expect("a");
        let (desk, cx) = start(first, data.path());
        open(&desk, &a, cx);
        let view = show(&desk, "a.md", cx);
        type_and_keep(
            &desk,
            &view,
            "Changed.
",
            cx,
        );
        std::fs::write(data.path().join("session.lock"), "open 4294967295").expect("a lock");
        let (desk, cx) = start(second, data.path());
        let copy = desk.read_with(cx, |desk, _| {
            let tab = desk
                .core
                .tabs
                .iter()
                .find(|tab| tab.session.path().is_none());
            tab.expect("an untitled copy").name.to_string()
        });
        let view = show(&desk, &copy, cx);
        let recovery = data.path().join("recovery");
        std::fs::remove_dir_all(&recovery).expect("removes");
        std::fs::write(&recovery, "not a folder").expect("blocks");
        type_and_keep(
            &desk, &view, "Again.
", cx,
        );
        assert_eq!(toasted(&desk, "was not kept for recovery", cx), 1);
        assert_eq!(
            toasted(&desk, "none yet", cx),
            0,
            "the recovered point's time shows"
        );
    }

    /// Every folder under `dir` read-only, or writable again.
    #[cfg(unix)]
    fn lock_down(dir: &std::path::Path, locked: bool) {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = if locked { 0o500 } else { 0o700 };
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(mode)).expect("permissions");
        for entry in std::fs::read_dir(dir).expect("lists") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                lock_down(&path, locked);
            }
        }
    }

    #[cfg(unix)]
    #[gpui::test]
    fn denied_writes_show_once_each(cx: &mut TestAppContext) {
        let (data, files) = (
            tempfile::tempdir().expect("data"),
            tempfile::tempdir().expect("files"),
        );
        let a = files.path().join("a.md");
        std::fs::write(
            &a, "Text.
",
        )
        .expect("a");
        let (desk, cx) = start(cx, data.path());
        open(&desk, &a, cx);
        let view = show(&desk, "a.md", cx);
        type_and_keep(
            &desk, &view, "One.
", cx,
        );
        lock_down(&data.path().join("recovery"), true);
        type_and_keep(
            &desk, &view, "Two.
", cx,
        );
        type_and_keep(
            &desk, &view, "Three.
", cx,
        );
        lock_down(&data.path().join("recovery"), false);
        assert_eq!(
            toasted(&desk, "was not kept for recovery", cx),
            1,
            "a point"
        );
        lock_down(data.path(), true);
        for line in [1, 2] {
            view.update(cx, |view, cx| {
                view.document()
                    .update(cx, |document, cx| document.set_caret(line, 0, cx))
            });
            desk.update(cx, |desk, cx| desk.keep(cx));
            cx.run_until_parked();
        }
        lock_down(data.path(), false);
        assert_eq!(
            toasted(&desk, "were not saved for the next launch", cx),
            1,
            "the session"
        );
    }

    #[gpui::test]
    fn a_cleared_recent_list_stays_cleared_with_tabs_restored(
        first: &mut TestAppContext,
        second: &mut TestAppContext,
    ) {
        let (data, files) = (
            tempfile::tempdir().expect("data"),
            tempfile::tempdir().expect("files"),
        );
        let (a, b) = (files.path().join("a.md"), files.path().join("b.md"));
        std::fs::write(&a, "A
").expect("a");
        std::fs::write(&b, "B
").expect("b");
        let (desk, cx) = start(first, data.path());
        open(&desk, &a, cx);
        let button = cx.debug_bounds("mark-clear-recent").expect("the button");
        cx.simulate_click(button.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        desk.update(cx, |desk, cx| {
            desk.leave(cx);
            desk.keeping.close();
        });
        cx.run_until_parked();

        let (desk, cx) = start(second, data.path());
        assert_eq!(names(&desk, cx), ["a.md"]);
        assert!(desk.read_with(cx, |desk, _| desk.recent.is_empty()));
        let list = data.path().join("recent.json");
        assert_eq!(std::fs::read_to_string(&list).expect("the list"), "[]");
        // Explicit opens still join it: the restored file, a link.
        let alias = files.path().join("alias.md");
        std::fs::hard_link(&a, &alias).expect("a hard link");
        open(&desk, &a, cx);
        open(&desk, &alias, cx);
        open(&desk, &b, cx);
        assert_eq!(names(&desk, cx), ["a.md", "b.md"]);
        let real = |path: &std::path::Path| dunce::canonicalize(path).expect("real");
        let want = [real(&b), real(&alias), real(&a)];
        assert_eq!(desk.read_with(cx, |desk, _| desk.recent.clone()), want);
        let kept: Vec<std::path::PathBuf> =
            serde_json::from_str(&std::fs::read_to_string(&list).expect("the list"))
                .expect("a list");
        assert_eq!(kept, want);
    }
}

mod off;
