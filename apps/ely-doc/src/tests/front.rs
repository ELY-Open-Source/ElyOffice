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

#[test]
fn a_chosen_load_that_fails_hands_the_front_on() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &docs);
    let (bad, good) = (dir.path().join("a.docx"), dir.path().join("b.fodt"));
    std::fs::write(&bad, b"not a package").expect("the file writes");
    std::fs::write(&good, cx.update(|app| Template::Report.fodt(app))).expect("the file writes");
    with_window(&mut cx, &docs, |docs, _, cx| {
        docs.open_file(bad, cx);
        let chosen = docs.core.loading.last().expect("a load in flight").0;
        docs.open_file(good, cx);
        // As a click on the first loading tab does.
        docs.core.set_active(Some(chosen));
    });
    settle(&cx, |app| docs.read(app).core.loading.is_empty() && docs.read(app).core.tabs.len() == 2);
    let landed = cx.update(|app| docs.read(app).core.tabs.iter().find(|tab| tab.name == "b.fodt").expect("opened").id);
    assert_eq!(cx.update(|app| docs.read(app).core.active()), Some(landed));
}

/// Whether tab `id`'s page has the keys in `window`.
fn focused(cx: &mut TestAppContext, docs: &Entity<Docs>, window: gpui::WindowHandle<Blank>, id: u64) -> bool {
    cx.update_window(window.into(), |_, window, app| {
        docs.read(app).tab(id).expect("the tab").view.read(app).focus_handle().is_focused(window)
    })
    .expect("the window lives")
}

/// A window that `docs` calls home.
fn home(cx: &mut TestAppContext, docs: &Entity<Docs>) -> gpui::WindowHandle<Blank> {
    let window = cx.add_window(|_, _| Blank);
    cx.update_window(window.into(), |_, window, app| docs.update(app, |docs, _| docs.core.window = Some(window.window_handle())))
        .expect("the window lives");
    window
}

#[test]
fn a_chosen_load_that_fails_gives_the_keys_back() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let typing = blank(&mut cx, &docs);
    let bad = dir.path().join("b.docx");
    std::fs::write(&bad, b"not a package").expect("the file writes");
    let window = home(&mut cx, &docs);
    cx.update(|app| {
        docs.update(app, |docs, cx| {
            docs.open_file(bad, cx);
            let chosen = docs.core.loading.last().expect("a load in flight").0;
            docs.core.set_active(Some(chosen));
        })
    });
    settle(&cx, |app| docs.read(app).core.loading.is_empty());
    assert_eq!(cx.update(|app| docs.read(app).core.active()), Some(typing));
    assert!(focused(&mut cx, &docs, window, typing), "the tab left in front has no keys");
}

#[test]
fn reopening_an_open_file_gives_it_the_keys() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let fodt = cx.update(|app| Template::Report.fodt(app));
    let (a, b) = (dir.path().join("a.fodt"), dir.path().join("b.fodt"));
    for file in [&a, &b] {
        std::fs::write(file, &fodt).expect("the file writes");
    }
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(a.clone(), cx));
    settle(&cx, |app| docs.read(app).core.tabs.len() == 1);
    let first = cx.update(|app| docs.read(app).core.tabs[0].id);
    let window = home(&mut cx, &docs);
    cx.update(|app| {
        docs.update(app, |docs, cx| {
            docs.open_file(b, cx);
            let loading = docs.core.loading.last().expect("a load in flight").0;
            docs.core.set_active(Some(loading));
            docs.open_file(a, cx);
        })
    });
    settle(&cx, |app| docs.read(app).core.tabs.len() == 2);
    assert_eq!(cx.update(|app| docs.read(app).core.active()), Some(first));
    assert!(focused(&mut cx, &docs, window, first), "the reopened file has no keys");
}

#[test]
fn reopening_a_file_still_loading_chooses_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &docs);
    let fodt = cx.update(|app| Template::Report.fodt(app));
    let (a, b) = (dir.path().join("a.fodt"), dir.path().join("b.fodt"));
    for file in [&a, &b] {
        std::fs::write(file, &fodt).expect("the file writes");
    }
    with_window(&mut cx, &docs, |docs, _, cx| {
        docs.open_file(b, cx);
        docs.open_file(a.clone(), cx);
        docs.open_file(a, cx);
        assert_eq!(docs.core.loading.len(), 2, "the reopen loads nothing more");
    });
    settle(&cx, |app| docs.read(app).core.tabs.len() == 3);
    let chosen = cx.update(|app| docs.read(app).core.tabs.iter().find(|tab| tab.name == "a.fodt").expect("opened").id);
    assert_eq!(cx.update(|app| docs.read(app).core.active()), Some(chosen));
}
