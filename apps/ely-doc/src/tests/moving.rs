//! A live document moves to a new window (W.1d).

use std::{cell::RefCell, rc::Rc};

use ely_office::{Core, moving::Moving as _};
use gpui::AnyWindowHandle;

use super::*;

/// The first window, built as the app builds it.
fn first(cx: &mut TestAppContext, dir: &Path) -> Option<(Entity<Docs>, AnyWindowHandle)> {
    // Real windows start recovery: never in the user's folder.
    ely_platform::test_root();
    ely_ui::logs::keep();
    let install = ely_lok::test_install()?;
    let (worker, stop) = ely_office::lok::test_worker(&install);
    let work = dir.join("work");
    std::fs::create_dir_all(&work).expect("the work dir");
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        ely_ui::settings::install_defaults(cx);
        ely_ui::install_theme(cx);
        ely_ui::i18n::install_as("en-US", cx);
        ely_ui::windows::allow_for_tests(ely_platform::AppId::Doc, crate::window::more, cx);
    });
    let held = Rc::new(RefCell::new(None));
    let kept = Rc::clone(&held);
    let window = cx
        .update(|cx| {
            cx.open_window(Default::default(), move |window, cx| {
                let docs = crate::window::docs(move |cx| Core::new(worker, stop, work, cx), cx);
                kept.replace(Some(docs.clone()));
                crate::window::root(&docs, window, cx)
            })
        })
        .expect("a window");
    let window: AnyWindowHandle = window.into();
    cx.update(|cx| ely_ui::tray::testing::opened(ely_platform::AppId::Doc, window, cx));
    let docs = held.take().expect("the docs");
    Some((docs, window))
}

/// Draws the windows until `done`; plain ticks draw nothing.
#[track_caller]
fn wait(cx: &mut TestAppContext, done: impl Fn(&gpui::App) -> bool) {
    // The worker answers in real time, not test time.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while std::time::Instant::now() < deadline {
        cx.run_until_parked();
        if cx.update(|app| done(app)) {
            return;
        }
        cx.executor().advance_clock(std::time::Duration::from_millis(20));
    }
    let kept = ely_ui::logs::kept();
    let tail: Vec<_> = kept.iter().filter(|line| ["leaves", "arrives", "stays", "it closes", "is gone", "no window"].iter().any(|word| line.contains(word))).collect();
    let tail = tail[tail.len().saturating_sub(12)..].iter().map(|line| line.as_str()).collect::<Vec<_>>();
    panic!("the windows never showed it; the log ends:\n{}", tail.join("\n"));
}

/// A blank document in `home`, its tab id.
fn opened(cx: &mut TestAppContext, docs: &Entity<Docs>, home: AnyWindowHandle) -> u64 {
    let before = cx.update(|app| docs.read(app).core.tabs.len());
    home.update(cx, |_, _, app| docs.update(app, |docs, cx| docs.new_from(Template::Blank, cx))).expect("home is open");
    wait(cx, |app| docs.read(app).core.tabs.len() > before);
    let id = cx.update(|app| docs.read(app).core.tabs.last().expect("a tab").id);
    wait(cx, |app| docs.read(app).tab(id).is_some_and(|tab| tab.view.read(app).settled()));
    id
}

#[test]
fn an_edited_document_moves_and_keeps_its_undo() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((docs, home)) = first(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    opened(&mut cx, &docs, home);
    let id = opened(&mut cx, &docs, home);
    type_into(&mut cx, &docs, "Moved words");
    let doc = cx.update(|app| docs.read(app).tab(id).expect("the tab").doc.clone());
    cx.update(|app| ely_office::moving::to_new_window(docs.clone(), id, app));
    wait(&mut cx, |app| docs.read(app).tab(id).is_none());
    let other = cx.update(|app| crate::window::front(None, app)).expect("a front window");
    assert_ne!(other, docs, "a new window is in front");
    cx.update(|app| {
        let tab = other.read(app).tab(id).expect("the moved tab");
        assert_eq!(tab.doc, doc, "the same kernel document, not reopened");
        assert!(tab.dirty(app), "its unsaved edit came along");
        assert_eq!(other.read(app).core.tabs.len(), 1, "no blank added");
    });
    other.update(&mut cx, |other, cx| other.run_active("edit.undo", &[], cx));
    wait(&mut cx, |app| other.read(app).tab(id).is_some_and(|tab| ely_office::undo_able(tab.view.read(app)).1));
    // The last tab leaves: its window closes.
    let last = cx.update(|app| docs.read(app).core.active()).expect("the other tab");
    cx.update(|app| ely_office::moving::to_new_window(docs.clone(), last, app));
    wait(&mut cx, |app| !app.windows().contains(&home));
}

#[test]
fn an_export_panel_keeps_its_tab() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((docs, home)) = first(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let id = opened(&mut cx, &docs, home);
    docs.update(&mut cx, |docs, cx| {
        docs.dialog = Some(crate::actions::Dialog::Output { tab: id, print: false, format: Default::default(), all: true, pdf: Default::default() });
        assert_eq!(docs.move_out(id, cx).err(), Some("office.move.dialog"), "its dialog is open");
        docs.output(cx);
    });
    assert!(cx.did_prompt_for_new_path(), "the export panel is open");
    let stays = docs.update(&mut cx, |docs, cx| docs.move_out(id, cx).err());
    assert_eq!(stays, Some("office.move.busy"), "the export lands here");
    cx.simulate_new_path_selection(|_| None);
    wait(&mut cx, |app| std::rc::Rc::strong_count(&docs.read(app).tab(id).expect("the tab").busy) == 1);
    assert!(docs.update(&mut cx, |docs, cx| docs.move_out(id, cx).is_ok()), "then it may move");
}

#[test]
fn a_word_count_in_flight_follows_its_tab() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((docs, home)) = first(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    opened(&mut cx, &docs, home);
    let id = opened(&mut cx, &docs, home);
    // One update: the move comes before the read can run.
    cx.update(|app| {
        docs.update(app, |docs, cx| {
            let view = docs.tab(id).expect("the tab").view.clone();
            let extra = &mut docs.tab_mut(id).expect("the tab").extra;
            (extra.counted, extra.counts) = (None, None);
            docs.count(id, &view, cx);
            assert!(docs.tab(id).expect("the tab").extra.counting, "the count is asked");
        });
        ely_office::moving::to_new_window(docs.clone(), id, app);
    });
    wait(&mut cx, |app| docs.read(app).tab(id).is_none());
    let other = cx.update(|app| crate::window::front(None, app)).expect("a front window");
    wait(&mut cx, |app| other.read(app).tab(id).is_some_and(|tab| !tab.extra.counting && tab.extra.counts.is_some()));
}
