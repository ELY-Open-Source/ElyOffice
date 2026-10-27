use std::{
    path::Path,
    sync::Arc,
};

use ely_office::prelude::*;
use gpui::{AppContext as _, Entity, TestAppContext};

use crate::deck::Deck;

mod menu;
mod motion;
mod notes;
mod objects;
mod output;
mod play;
mod play_more;
mod play_time;
mod remote;
mod review;
mod slides;
mod strip;
mod turn;
// Atlas pictures; macOS draws surfaces on its main thread.
#[cfg(not(target_os = "macos"))]
mod video;

/// An unrendered Deck over a fresh worker, if LOK exists.
pub(super) fn deck(cx: &mut TestAppContext, dir: &Path) -> Option<Entity<Deck>> {
    let install = ely_lok::install_dir().ok()?;
    let worker = ely_office::lok::test_worker(&install, &dir.join("profile"));
    let work = dir.join("work");
    std::fs::create_dir_all(&work).expect("the work dir");
    cx.update(ely_gpui_component::init_for_tests);
    cx.update(|cx| {
        ely_ui::settings::install_defaults(cx);
        ely_ui::i18n::install(cx);
    });
    let window = cx.add_window(|_, _| gpui::Empty);
    let deck = cx
        .update_window(window.into(), |_, window, app| {
            let deck = app.new(|cx| Deck::new(worker, work, cx));
            deck.update(app, |deck, _| {
                deck.core.window = Some(window.window_handle())
            });
            deck
        })
        .expect("the window lives");
    Some(deck)
}

/// Steps the scheduler until `done` or ten seconds pass.
#[track_caller]
pub(super) fn settle(cx: &TestAppContext, done: impl Fn(&gpui::App) -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if !cx.executor().tick() {
            cx.executor()
                .advance_clock(std::time::Duration::from_millis(20));
        }
        if cx.update(|app| done(app)) {
            return;
        }
    }
    panic!("the state never settled");
}

pub(super) fn update(
    cx: &mut TestAppContext,
    deck: &Entity<Deck>,
    f: impl FnOnce(&mut Deck, &mut gpui::Context<Deck>),
) {
    cx.update(|app| deck.update(app, f));
}

/// Slides and painted pictures of the active deck.
fn counts(app: &gpui::App, deck: &Entity<Deck>) -> (usize, usize) {
    deck.read(app).active_tab().map_or((0, 0), |tab| {
        let painted = tab
            .extra
            .slides
            .list
            .iter()
            .filter(|slide| {
                tab.extra.thumbs.contains_key(&slide.name) && !tab.extra.stale.contains(&slide.name)
            })
            .count();
        (tab.extra.slides.list.len(), painted)
    })
}

fn quiet(cx: &TestAppContext, deck: &Entity<Deck>) {
    settle(cx, |app| {
        deck.read(app).active_tab().is_some_and(|tab| {
            tab.view.read(app).settled() && !tab.extra.reading && !tab.extra.painting
        })
    });
}

#[test]
fn a_blank_deck_grows_and_every_slide_has_a_picture() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    update(&mut cx, &deck, |deck, cx| deck.new_blank(cx));
    settle(&cx, |app| counts(app, &deck) == (1, 1));
    let layout = cx.update(|app| {
        deck.read(app)
            .active_tab()
            .and_then(|tab| tab.extra.design.clone())
    });
    assert_eq!(layout.expect("a design").layout, 0, "a title slide");
    update(&mut cx, &deck, |deck, cx| {
        deck.run_active("slide.insert", &[], cx)
    });
    update(&mut cx, &deck, |deck, cx| {
        deck.run_active("slide.duplicate", &[], cx)
    });
    settle(&cx, |app| counts(app, &deck) == (3, 3));
    update(&mut cx, &deck, |deck, cx| {
        deck.run_active("slide.hide", &[], cx)
    });
    settle(&cx, |app| {
        deck.read(app)
            .active_tab()
            .is_some_and(|tab| tab.extra.slides.list.iter().any(|slide| !slide.visible))
    });
}

#[test]
fn design_changes_reach_the_kernel_and_repaint() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    update(&mut cx, &deck, |deck, cx| deck.new_blank(cx));
    settle(&cx, |app| counts(app, &deck) == (1, 1));
    let before = cx.update(|app| {
        deck.read(app)
            .active_tab()
            .expect("a tab")
            .extra
            .thumbs
            .values()
            .next()
            .cloned()
    });
    update(&mut cx, &deck, |deck, cx| {
        deck.design_command("slide.size", &["25400", "19050", "true"], cx)
    });
    settle(&cx, |app| {
        deck.read(app).active_tab().is_some_and(|tab| {
            let (w, h) = tab.extra.size;
            (w - 14400).abs() <= 2 && (h - 10800).abs() <= 2
        })
    });
    quiet(&cx, &deck);
    let after = cx.update(|app| {
        deck.read(app)
            .active_tab()
            .expect("a tab")
            .extra
            .thumbs
            .values()
            .next()
            .cloned()
    });
    assert!(
        !Arc::ptr_eq(&before.expect("a picture"), &after.expect("a picture")),
        "repainted"
    );
    update(&mut cx, &deck, |deck, cx| {
        deck.design_command("slide.layout", &["20"], cx)
    });
    settle(&cx, |app| {
        deck.read(app)
            .active_tab()
            .and_then(|tab| tab.extra.design.as_ref())
            .is_some_and(|design| design.layout == 20)
    });
}

#[test]
fn templates_open_as_untitled_decks() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let templates = ely_lok::templates("presnt").expect("bundled templates");
    assert!(templates.len() >= 5, "{templates:?}");
    update(&mut cx, &deck, |deck, cx| {
        deck.new_from_file(&templates[0], cx)
    });
    settle(&cx, |app| counts(app, &deck).0 >= 1);
    let name = cx.update(|app| deck.read(app).active_tab().expect("a tab").name.clone());
    assert!(
        name.contains("Untitled") || name.contains("未命名"),
        "{name}"
    );
}

#[test]
fn the_master_view_comes_and_goes() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    update(&mut cx, &deck, |deck, cx| deck.new_blank(cx));
    settle(&cx, |app| counts(app, &deck) == (1, 1));
    let masters = |app: &gpui::App| {
        deck.read(app)
            .active_tab()
            .is_some_and(|tab| tab.extra.masters)
    };
    update(&mut cx, &deck, |deck, cx| deck.masters(true, cx));
    settle(&cx, |app| masters(app) && counts(app, &deck) == (1, 1));
    let shown = cx.update(|app| {
        deck.read(app)
            .active_tab()
            .expect("a tab")
            .extra
            .slides
            .list[0]
            .name
            .clone()
    });
    assert_eq!(shown, "Default", "the strip lists masters");
    update(&mut cx, &deck, |deck, cx| deck.masters(false, cx));
    settle(&cx, |app| !masters(app) && counts(app, &deck) == (1, 1));
}

/// Each cached picture equals the kernel's fresh one.
fn pictures_match(cx: &mut TestAppContext, deck: &Entity<Deck>) {
    quiet(cx, deck);
    cx.update(|app| {
        let deck = deck.read(app);
        let tab = deck.active_tab().expect("a tab");
        let mut worker = deck.core.worker.lock().expect("the worker lock");
        for (part, slide) in tab.extra.slides.list.iter().enumerate() {
            let fresh = tab
                .doc
                .paint_part(&mut worker, (part, false), (320, 180))
                .expect("paints");
            let cached = tab.extra.thumbs.get(&slide.name).expect("a picture");
            assert_eq!(
                cached.as_bytes(0),
                Some(&fresh[..]),
                "slide {part} is current"
            );
        }
    });
}

#[test]
fn undoing_a_resize_repaints_every_slide() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    update(&mut cx, &deck, |deck, cx| deck.new_blank(cx));
    settle(&cx, |app| counts(app, &deck) == (1, 1));
    update(&mut cx, &deck, |deck, cx| {
        deck.run_active("slide.duplicate", &[], cx)
    });
    settle(&cx, |app| counts(app, &deck) == (2, 2));
    // Width then height, before either reads back.
    update(&mut cx, &deck, |deck, cx| deck.resize((25_400, 15_750), cx));
    update(&mut cx, &deck, |deck, cx| {
        let (w, _) = deck.active_tab().expect("a tab").extra.shown_size();
        deck.resize((w, 19_050), cx)
    });
    settle(&cx, |app| {
        deck.read(app).active_tab().is_some_and(|tab| {
            let (w, h) = crate::design::mm100(tab.extra.size);
            tab.extra.size_asked.is_none() && (w - 25_400).abs() <= 2 && (h - 19_050).abs() <= 2
        })
    });
    pictures_match(&mut cx, &deck);
    update(&mut cx, &deck, |deck, cx| {
        deck.run_active("edit.undo", &[], cx)
    });
    update(&mut cx, &deck, |deck, cx| {
        deck.run_active("edit.undo", &[], cx)
    });
    settle(&cx, |app| {
        deck.read(app).active_tab().is_some_and(|tab| {
            let (w, h) = crate::design::mm100(tab.extra.size);
            (w - 28_000).abs() <= 2 && (h - 15_750).abs() <= 2
        })
    });
    pictures_match(&mut cx, &deck);
}

#[test]
fn a_failed_picture_waits_for_the_next_read() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    update(&mut cx, &deck, |deck, cx| deck.new_blank(cx));
    settle(&cx, |app| counts(app, &deck) == (1, 1));
    // The kernel loses the document under the deck.
    cx.update(|app| {
        let deck = deck.read(app);
        let doc = deck.active_tab().expect("a tab").doc.clone();
        deck.core
            .worker
            .lock()
            .expect("the worker lock")
            .close_doc(doc)
            .expect("closes");
    });
    update(&mut cx, &deck, |deck, cx| {
        let id = deck.core.active.expect("a tab");
        deck.tab_mut(id).expect("the tab").extra.thumbs.clear();
        deck.paint_next(id, cx);
    });
    cx.run_until_parked();
    let failed = cx.update(|app| {
        deck.read(app)
            .active_tab()
            .expect("a tab")
            .extra
            .failed
            .len()
    });
    assert_eq!(failed, 1, "one failure, then no more requests");
}

/// Undo steps the kernel holds for the active deck.
fn undo_steps(cx: &mut TestAppContext, deck: &Entity<Deck>) -> usize {
    quiet(cx, deck);
    cx.update(|app| {
        let deck = deck.read(app);
        let tab = deck.active_tab().expect("a tab");
        let mut worker = deck.core.worker.lock().expect("the worker lock");
        let json: serde_json::Value =
            serde_json::from_str(&worker.values(&tab.doc, ".uno:Undo").expect("reads"))
                .expect("parses");
        json["actions"].as_array().map_or(0, Vec::len)
    })
}

#[test]
fn the_same_size_twice_is_one_step() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    update(&mut cx, &deck, |deck, cx| deck.new_blank(cx));
    settle(&cx, |app| counts(app, &deck) == (1, 1));
    let before = undo_steps(&mut cx, &deck);
    // Enter, then the blur that follows it.
    update(&mut cx, &deck, |deck, cx| deck.resize((30_000, 15_750), cx));
    update(&mut cx, &deck, |deck, cx| deck.resize((30_000, 15_750), cx));
    settle(&cx, |app| {
        deck.read(app)
            .active_tab()
            .is_some_and(|tab| tab.extra.size_asked.is_none())
    });
    assert_eq!(undo_steps(&mut cx, &deck), before + 1);
}

/// Two slides whose master shows MasterBrand.
fn branded(cx: &mut TestAppContext, dir: &Path) -> Option<Entity<Deck>> {
    let deck = deck(cx, dir)?;
    update(cx, &deck, |deck, cx| deck.new_blank(cx));
    settle(cx, |app| counts(app, &deck) == (1, 1));
    update(cx, &deck, |deck, cx| {
        deck.run_active("slide.duplicate", &[], cx)
    });
    settle(cx, |app| counts(app, &deck) == (2, 2));
    let masters = |app: &gpui::App| {
        deck.read(app)
            .active_tab()
            .is_some_and(|tab| tab.extra.masters)
    };
    update(cx, &deck, |deck, cx| deck.masters(true, cx));
    settle(cx, |app| masters(app) && counts(app, &deck) == (1, 1));
    cx.update(|app| {
        let deck = deck.read(app);
        let doc = deck.active_tab().expect("a tab").doc.clone();
        let mut worker = deck.core.worker.lock().expect("the worker lock");
        worker
            .paste(&doc, "text/plain;charset=utf-8", b"MasterBrand")
            .expect("pastes");
    });
    update(cx, &deck, |deck, cx| deck.masters(false, cx));
    settle(cx, |app| !masters(app) && counts(app, &deck) == (2, 2));
    pictures_match(cx, &deck);
    Some(deck)
}

/// Runs both commands in one update.
fn both(cx: &mut TestAppContext, deck: &Entity<Deck>, first: &str, then: &str) {
    update(cx, deck, |deck, cx| {
        deck.run_active(first, &[], cx);
        deck.run_active(then, &[], cx);
    });
    settle(cx, |app| counts(app, deck) == (2, 2));
    pictures_match(cx, deck);
}

#[test]
fn undoing_a_master_edit_repaints_its_slides() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = branded(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    update(&mut cx, &deck, |deck, cx| {
        deck.run_active("edit.undo", &[], cx)
    });
    settle(&cx, |app| counts(app, &deck) == (2, 2));
    pictures_match(&mut cx, &deck);
    both(&mut cx, &deck, "edit.redo", "slide.hide");
}

#[test]
fn an_edit_right_after_undo_still_repaints() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = branded(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    both(&mut cx, &deck, "edit.undo", "slide.hide");
}

#[test]
fn leaving_text_edit_repaints_the_picture() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    update(&mut cx, &deck, |deck, cx| deck.new_blank(cx));
    settle(&cx, |app| counts(app, &deck) == (1, 1));
    let caret = |app: &gpui::App| {
        deck.read(app)
            .active_tab()
            .is_some_and(|tab| tab.view.read(app).caret())
    };
    cx.update(|app| {
        let deck = deck.read(app);
        let doc = deck.active_tab().expect("a tab").doc.clone();
        let mut worker = deck.core.worker.lock().expect("the worker lock");
        for kind in [0, 1] {
            let click = ely_lok::Mouse {
                kind,
                x: 7900,
                y: 1100,
                count: 1,
                buttons: 1,
                modifiers: 0,
            };
            worker.post_mouse(&doc, click).expect("clicks");
        }
        for c in "Plan".chars() {
            worker.post_key(&doc, c as u32, 0).expect("types");
        }
    });
    settle(&cx, caret);
    quiet(&cx, &deck);
    cx.update(|app| {
        let deck = deck.read(app);
        let doc = deck.active_tab().expect("a tab").doc.clone();
        let mut worker = deck.core.worker.lock().expect("the worker lock");
        worker.post_key(&doc, 0, 1281).expect("escapes");
    });
    settle(&cx, |app| !caret(app));
    pictures_match(&mut cx, &deck);
}
