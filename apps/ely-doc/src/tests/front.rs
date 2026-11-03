//! Loads in flight take the front only when asked last.

use std::{cell::RefCell, rc::Rc};

use gpui::TestAppContext;

use super::*;

#[test]
fn only_the_last_file_asked_takes_the_front() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let typing = blank(&mut cx, &docs);
    let fodt = cx.update(|app| Template::Report.fodt(app));
    let files: Vec<_> = ["a.fodt", "b.fodt", "c.fodt"].iter().map(|name| dir.path().join(name)).collect();
    for file in &files {
        std::fs::write(file, &fodt).expect("the file writes");
    }
    let seen = Rc::new(RefCell::new(Vec::new()));
    let watched = Rc::clone(&seen);
    cx.update(|app| app.observe(&docs, move |docs, cx| watched.borrow_mut().push(docs.read(cx).core.active())).detach());
    with_window(&mut cx, &docs, |docs, _, cx| files.iter().for_each(|file| docs.open_file(file.clone(), cx)));
    settle(&cx, |app| docs.read(app).core.tabs.len() == 4);
    let named = |name: &str| cx.update(|app| docs.read(app).core.tabs.iter().find(|tab| tab.name == name).expect("opened").id);
    let last = named("c.fodt");
    let fronts: Vec<_> = seen.borrow().iter().flatten().copied().filter(|id| *id != typing).collect();
    assert!(fronts.iter().all(|id| *id == last), "the front went to {fronts:?}, not only {last}");
    assert_eq!(cx.update(|app| docs.read(app).core.active()), Some(last));
}

#[test]
fn going_back_during_a_load_keeps_the_tab_chosen() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let typing = blank(&mut cx, &docs);
    let file = dir.path().join("c.fodt");
    std::fs::write(&file, cx.update(|app| Template::Report.fodt(app))).expect("the file writes");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.open_file(file, cx);
        assert_eq!(docs.core.tabs.len(), 1, "the load has not landed");
        docs.back.push((typing, (0, 0, 0, 0)));
        docs.on_back(&crate::navigate::GoBack, window, cx);
    });
    settle(&cx, |app| docs.read(app).core.tabs.len() == 2);
    assert_eq!(cx.update(|app| docs.read(app).core.active()), Some(typing));
}

#[test]
fn a_loading_tab_chosen_takes_focus_when_it_lands() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &docs);
    let file = dir.path().join("c.fodt");
    std::fs::write(&file, cx.update(|app| Template::Report.fodt(app))).expect("the file writes");
    let window = cx.add_window(|_, _| Blank);
    cx.update_window(window.into(), |_, window, app| {
        docs.update(app, |docs, cx| {
            docs.core.window = Some(window.window_handle());
            docs.open_file(file, cx);
            // As a click on its loading tab does.
            let loading = docs.core.loading.last().expect("a load in flight").0;
            docs.core.set_active(Some(loading));
        })
    })
    .expect("the window lives");
    settle(&cx, |app| docs.read(app).core.tabs.len() == 2);
    let focused = cx
        .update_window(window.into(), |_, window, app| {
            docs.read(app).active_tab().expect("a tab").view.read(app).focus_handle().is_focused(window)
        })
        .expect("the window lives");
    assert!(focused, "the chosen tab landed without focus");
}
