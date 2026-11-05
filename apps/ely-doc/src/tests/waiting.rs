//! Files behind the front load only when shown (R15.57).

use std::{cell::RefCell, path::PathBuf};

use gpui::TestAppContext;

use super::*;

/// Tab or load `name`: its id, and whether loaded.
fn named(cx: &mut TestAppContext, docs: &Entity<Docs>, name: &str) -> (u64, bool) {
    cx.update(|app| {
        let core = &docs.read(app).core;
        let tab = core.tabs.iter().find(|tab| tab.name == name).map(|tab| (tab.id, true));
        let load = || core.loading.iter().find(|(_, of, _)| of == name).map(|(id, ..)| (*id, false));
        tab.or_else(load).expect("a tab or a load")
    })
}

/// Three files at once behind a tab being typed in.
fn three_behind(cx: &mut TestAppContext, dir: &Path) -> Option<(Entity<Docs>, Vec<PathBuf>)> {
    let docs = docs(cx, dir)?;
    blank(cx, &docs);
    let fodt = cx.update(|app| Template::Report.fodt(app));
    let files: Vec<_> = ["a.fodt", "b.fodt", "c.fodt"].iter().map(|name| dir.join(name)).collect();
    for file in &files {
        std::fs::write(file, &fodt).expect("the file writes");
    }
    with_window(cx, &docs, |docs, _, cx| files.iter().for_each(|file| docs.open_file(file.clone(), cx)));
    settle(cx, |app| docs.read(app).core.tabs.iter().any(|tab| tab.name == "c.fodt"));
    cx.run_until_parked();
    Some((docs, files))
}

#[test]
fn files_opened_behind_the_front_load_only_when_shown() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((docs, files)) = three_behind(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let (c, _) = named(&mut cx, &docs, "c.fodt");
    let (a, a_loaded) = named(&mut cx, &docs, "a.fodt");
    let (b, b_loaded) = named(&mut cx, &docs, "b.fodt");
    assert!(!a_loaded && !b_loaded, "a and b loaded behind the front");
    assert_eq!(cx.update(|app| docs.read(app).core.active()), Some(c));
    // Shown: it loads and takes the keys.
    with_window(&mut cx, &docs, |docs, window, cx| docs.show_tab(a, window, cx));
    settle(&cx, |app| docs.read(app).core.tabs.iter().any(|tab| tab.id == a));
    assert_eq!(cx.update(|app| docs.read(app).core.active()), Some(a));
    assert!(!named(&mut cx, &docs, "b.fodt").1, "b stays waiting");
    // Closed unloaded, it can open again.
    with_window(&mut cx, &docs, |docs, window, cx| docs.request_close(b, After::Tab, window, cx));
    assert!(cx.update(|app| docs.read(app).core.loading.is_empty()), "b still waits");
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(files[1].clone(), cx));
    settle(&cx, |app| docs.read(app).core.tabs.iter().any(|tab| tab.name == "b.fodt"));
}

#[test]
fn a_program_asking_for_a_waiting_file_loads_it_first() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((docs, files)) = three_behind(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let answers = docs.clone();
    cx.update(|app| ely_ui::on_ask(app, move |ask, cx| answers.update(cx, |docs, cx| docs.answer_request(ask, cx))));
    let (c, _) = named(&mut cx, &docs, "c.fodt");
    let ask = |cx: &mut TestAppContext, words: serde_json::Value| {
        let (ask, answered) = ely_ui::Ask::for_tests(&words.to_string());
        cx.update(|app| docs.update(app, |docs, cx| docs.answer_request(ask, cx)));
        let answer = RefCell::new(None);
        settle(cx, |_| {
            let mut got = answer.borrow_mut();
            if got.is_none() {
                *got = answered.try_recv().ok();
            }
            got.is_some()
        });
        answer.into_inner().expect("answered")
    };
    let listed = ask(&mut cx, serde_json::json!(["docs"])).expect("docs");
    let a = files[0].display().to_string();
    assert!(listed.lines().any(|line| line == a), "{a} not in {listed}");
    let text = ask(&mut cx, serde_json::json!(["get", a])).expect("a's text");
    assert!(!text.is_empty(), "a reads empty");
    assert!(named(&mut cx, &docs, "a.fodt").1, "a loaded for the program");
    assert_eq!(cx.update(|app| docs.read(app).core.active()), Some(c), "the program took the front");
}
