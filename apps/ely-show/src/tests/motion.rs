//! Transitions, effects and their previews from the deck (C10.4).

use ely_lok::show::{Part, Transition};
use ely_office::prelude::*;
use gpui::{AppContext as _, TestAppContext};

use super::objects::{act, blank};
use super::{settle, update};
use crate::preview::Playing;

#[test]
fn a_transition_and_an_effect_land_and_preview() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let motion = |app: &gpui::App| {
        deck.read(app)
            .active_tab()
            .and_then(|tab| tab.extra.motion.clone())
    };
    settle(&cx, |app| motion(app).is_some());
    let fade = Transition {
        preset: "fade-smoothly".into(),
        duration: 0.5,
        auto: false,
        time: 0.0,
    };
    let json = ely_lok::show::transition_json(&fade, false);
    update(&mut cx, &deck, |deck, cx| {
        deck.design_command("slide.transition", &[&json], cx)
    });
    settle(&cx, |app| motion(app).is_some_and(|(t, _)| t == fade));
    act(
        &mut cx,
        &deck,
        ("draw.shape", &["rectangle", "true"]),
        |o| o.selection.is_some(),
    );
    let add = serde_json::json!({ "preset": "ooo-entrance-fly-in" }).to_string();
    update(&mut cx, &deck, |deck, cx| {
        deck.design_command("animation.add", &[&add], cx)
    });
    settle(&cx, |app| {
        motion(app).is_some_and(|(_, effects)| effects.len() == 1)
    });
    let hash = cx.update(|app| motion(app).expect("read").1[0].hash.clone());
    update(&mut cx, &deck, |deck, cx| deck.preview_effects(cx));
    settle(&cx, |app| deck.read(app).preview.is_some());
    let moving =
        cx.update(
            |app| match &deck.read(app).preview.as_ref().expect("playing").playing {
                Playing::Effects { layers, .. } => layers
                    .iter()
                    .filter(|(part, _)| matches!(part, Part::Moving { hash: h, .. } if *h == hash))
                    .count(),
                Playing::Transition { .. } => 0,
            },
        );
    assert_eq!(moving, 1, "the shape plays as its own layer");
    update(&mut cx, &deck, |deck, cx| deck.stop_preview(cx));
    update(&mut cx, &deck, |deck, cx| deck.preview_transition(cx));
    settle(&cx, |app| deck.read(app).preview.is_some());
    let first = cx.update(
        |app| match &deck.read(app).preview.as_ref().expect("playing").playing {
            Playing::Transition { from, preset, .. } => (from.is_none(), preset.clone()),
            Playing::Effects { .. } => (false, String::new()),
        },
    );
    assert_eq!(
        first,
        (true, "fade-smoothly".to_string()),
        "the first slide comes from black"
    );
}

#[test]
fn inserted_media_reads_back_and_is_picked() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let clip = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/media/clip.mp4");
    let url = ely_lok::file_url(&clip);
    update(&mut cx, &deck, |deck, cx| {
        let tab = deck.core.active();
        deck.run_in(tab, "media.insert", &[&url, "false", "8000", "4500"], cx);
        deck.force_reread(tab.expect("a tab"), cx);
    });
    let media = |app: &gpui::App| {
        deck.read(app)
            .active_tab()
            .map(|tab| tab.extra.media.clone())
            .unwrap_or_default()
    };
    settle(&cx, |app| media(app).len() == 1);
    let index = cx.update(|app| media(app)[0].index).to_string();
    act(&mut cx, &deck, ("object.select", &[&index, "false"]), |o| {
        o.selection.is_some()
    });
    let picked = cx.update(|app| deck.read(app).picked_media());
    assert!(picked.is_some_and(|(_, media)| !media.missing && !media.link));
}

#[test]
fn a_stop_cancels_a_preview_on_its_way_and_a_slide_change_ends_one() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let ready = |app: &gpui::App| {
        deck.read(app)
            .active_tab()
            .is_some_and(|tab| tab.extra.motion.is_some() && tab.extra.history.is_some())
    };
    settle(&cx, ready);
    update(&mut cx, &deck, |deck, cx| {
        deck.preview_transition(cx);
        deck.stop_preview(cx);
    });
    for _ in 0..100 {
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(20));
        cx.run_until_parked();
    }
    assert!(
        cx.update(|app| deck.read(app).preview.is_none()),
        "the stopped read never plays"
    );
    update(&mut cx, &deck, |deck, cx| deck.preview_transition(cx));
    settle(&cx, |app| deck.read(app).preview.is_some());
    update(&mut cx, &deck, |deck, cx| {
        let tab = deck.core.active();
        deck.run_in(tab, "slide.insert", &[], cx);
        deck.force_reread(tab.expect("a tab"), cx);
    });
    settle(&cx, |app| {
        deck.read(app)
            .active_tab()
            .is_some_and(|tab| tab.extra.slides.current == 1)
    });
    update(&mut cx, &deck, |deck, cx| deck.drop_stale_preview(cx));
    assert!(
        cx.update(|app| deck.read(app).preview.is_none()),
        "another slide ends it"
    );
}

#[test]
fn typing_ends_a_preview_before_any_undo_step() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    act(&mut cx, &deck, ("object.select", &["0", "false"]), |o| {
        o.selection.is_some()
    });
    let view = cx.update(|app| deck.read(app).active_tab().expect("a tab").view.clone());
    let typed = |cx: &mut TestAppContext, text: &str| {
        let before = cx.update(|app| view.read(app).edited());
        cx.update(|app| view.update(app, |view, _| view.insert(text)));
        settle(cx, |app| {
            view.read(app).edited() > before && view.read(app).settled()
        });
    };
    typed(&mut cx, "A");
    let ready = |app: &gpui::App| {
        let tab = deck.read(app).active_tab();
        tab.is_some_and(|tab| tab.extra.motion.is_some() && tab.extra.history.is_some())
    };
    settle(&cx, ready);
    update(&mut cx, &deck, |deck, cx| deck.preview_transition(cx));
    settle(&cx, |app| deck.read(app).preview.is_some());
    let undo = |app: &mut gpui::App| deck.read(app).active_tab()?.extra.history.clone();
    let steps = cx.update(undo);
    typed(&mut cx, "B");
    assert_eq!(cx.update(undo), steps, "no undo step yet");
    update(&mut cx, &deck, |deck, cx| deck.drop_stale_preview(cx));
    assert!(
        cx.update(|app| deck.read(app).preview.is_none()),
        "the typed text ends it"
    );
}

#[test]
fn enter_then_blur_sends_one_duration() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let motion = |app: &gpui::App| {
        deck.read(app)
            .active_tab()
            .and_then(|tab| tab.extra.motion.clone())
    };
    settle(&cx, |app| motion(app).is_some());
    let panel = cx.update(|app| {
        let weak = deck.downgrade();
        app.new(|cx| crate::design::Panel::new(weak, cx))
    });
    let now = cx.update(|app| motion(app).expect("read").0);
    let send = crate::motion::dedupe(panel, "transition.duration", now.duration, |deck, n, cx| {
        deck.send_transition(|t| t.duration = n, false, cx)
    });
    let steps = super::undo_steps(&mut cx, &deck);
    let handle = cx
        .update(|app| deck.read(app).core.window)
        .expect("a window");
    for _ in 0..2 {
        cx.update_window(handle, |_, window, app| send(3.5, window, app))
            .expect("the window lives");
    }
    settle(&cx, |app| {
        motion(app).is_some_and(|(t, _)| t.duration == 3.5)
    });
    assert_eq!(
        super::undo_steps(&mut cx, &deck),
        steps + 1,
        "one step for Enter and blur"
    );
}

#[test]
fn two_fields_sent_before_a_read_both_land() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let motion = |app: &gpui::App| {
        deck.read(app)
            .active_tab()
            .and_then(|tab| tab.extra.motion.clone())
    };
    settle(&cx, |app| motion(app).is_some());
    update(&mut cx, &deck, |deck, cx| {
        deck.send_transition(
            |t| (t.auto, t.duration, t.time) = (true, 2.0, 4.0),
            false,
            cx,
        )
    });
    settle(&cx, |app| {
        motion(app).is_some_and(|(t, _)| t.auto && t.time == 4.0)
    });
    let panel = cx.update(|app| {
        let weak = deck.downgrade();
        app.new(|cx| crate::design::Panel::new(weak, cx))
    });
    let duration =
        crate::motion::dedupe(panel.clone(), "transition.duration", 2.0, |deck, n, cx| {
            deck.send_transition(|t| t.duration = n, false, cx)
        });
    let time = crate::motion::dedupe(panel, "transition.time", 4.0, |deck, n, cx| {
        deck.send_transition(|t| t.time = n, false, cx)
    });
    let handle = cx
        .update(|app| deck.read(app).core.window)
        .expect("a window");
    cx.update_window(handle, |_, window, app| {
        duration(3.5, window, app);
        time(7.0, window, app);
    })
    .expect("the window lives");
    settle(&cx, |app| {
        let asked = deck
            .read(app)
            .active_tab()
            .map(|t| t.extra.transition_asked.is_none());
        asked == Some(true) && motion(app).is_some_and(|(t, _)| t.time == 7.0)
    });
    let read = cx.update(|app| motion(app).expect("read").0);
    assert_eq!((read.duration, read.time), (3.5, 7.0));
}

#[test]
fn an_undone_duration_is_not_sent_again() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let motion = |app: &gpui::App| {
        deck.read(app)
            .active_tab()
            .and_then(|tab| tab.extra.motion.clone())
    };
    let asked = |app: &gpui::App| {
        let tab = deck.read(app).active_tab();
        tab.is_some_and(|tab| tab.extra.transition_asked.is_some())
    };
    settle(&cx, |app| motion(app).is_some());
    update(&mut cx, &deck, |deck, cx| {
        deck.send_transition(
            |t| (t.auto, t.duration, t.time) = (true, 2.0, 4.0),
            false,
            cx,
        )
    });
    settle(&cx, |app| {
        !asked(app) && motion(app).is_some_and(|(t, _)| t.time == 4.0)
    });
    update(&mut cx, &deck, |deck, cx| {
        deck.send_transition(|t| t.duration = 3.5, false, cx);
        let tab = deck.core.active();
        deck.run_in(tab, "edit.undo", &[], cx);
        deck.force_reread(tab.expect("a tab"), cx);
    });
    settle(&cx, |app| !asked(app));
    update(&mut cx, &deck, |deck, cx| {
        deck.send_transition(|t| t.time = 7.0, false, cx)
    });
    settle(&cx, |app| {
        !asked(app) && motion(app).is_some_and(|(t, _)| t.time == 7.0)
    });
    let read = cx.update(|app| motion(app).expect("read").0);
    assert_eq!((read.duration, read.time), (2.0, 7.0));
}

#[test]
fn two_effects_take_the_same_value_before_a_read() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let effects = |app: &gpui::App| {
        deck.read(app)
            .active_tab()
            .and_then(|tab| tab.extra.motion.clone())
            .map(|(_, effects)| effects)
            .unwrap_or_default()
    };
    act(
        &mut cx,
        &deck,
        ("draw.shape", &["rectangle", "true"]),
        |o| o.selection.is_some(),
    );
    let add = serde_json::json!({ "preset": "ooo-entrance-fade-in" }).to_string();
    for count in 1..=2 {
        update(&mut cx, &deck, |deck, cx| {
            deck.design_command("animation.add", &[&add], cx)
        });
        settle(&cx, |app| effects(app).len() == count);
    }
    for at in 0..2 {
        update(&mut cx, &deck, |deck, cx| {
            deck.send_effect(at, serde_json::json!({ "duration": 2.0 }), cx)
        });
    }
    settle(&cx, |app| effects(app).iter().all(|e| e.duration == 2.0));
    let panel = cx.update(|app| {
        let weak = deck.downgrade();
        app.new(|cx| crate::design::Panel::new(weak, cx))
    });
    let (slide, listed) = cx.update(|app| {
        let tab = deck.read(app).active_tab().expect("a tab");
        let slide = tab.extra.slides.list[tab.extra.slides.current].name.clone();
        (slide, effects(app))
    });
    let handle = cx
        .update(|app| deck.read(app).core.window)
        .expect("a window");
    let steps = super::undo_steps(&mut cx, &deck);
    cx.update_window(handle, |_, window, app| {
        for (at, effect) in listed.iter().enumerate() {
            let key = crate::motion::effect_key("duration", &slide, (at, effect));
            let send = crate::motion::dedupe(panel.clone(), key, 2.0, move |deck, n, cx| {
                deck.send_effect(at, serde_json::json!({ "duration": n }), cx)
            });
            // Enter, then blur.
            send(3.5, window, app);
            send(3.5, window, app);
        }
    })
    .expect("the window lives");
    settle(&cx, |app| effects(app).iter().all(|e| e.duration == 3.5));
    assert_eq!(
        super::undo_steps(&mut cx, &deck),
        steps + 2,
        "one step each"
    );
}
