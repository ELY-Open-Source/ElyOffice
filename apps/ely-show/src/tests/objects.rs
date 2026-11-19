use ely_lok::show::{GuideKind, Objects, Paint, Selection};
use ely_office::prelude::*;
use gpui::{Entity, TestAppContext};

use super::{counts, deck, pictures_match, settle, undo_steps, update};
use crate::deck::Deck;

pub(super) fn objects(app: &gpui::App, deck: &Entity<Deck>) -> Option<Objects> {
    deck.read(app).active_tab()?.extra.objects.clone()
}

pub(super) fn selection(app: &gpui::App, deck: &Entity<Deck>) -> Option<Selection> {
    objects(app, deck)?.selection
}

/// A blank deck, its first picture painted.
pub(crate) fn blank(cx: &mut TestAppContext, dir: &std::path::Path) -> Option<Entity<Deck>> {
    let deck = deck(cx, dir)?;
    update(cx, &deck, |deck, cx| deck.new_blank(cx));
    settle(cx, |app| counts(app, &deck) == (1, 1));
    Some(deck)
}

/// Runs `id` and waits until `done` holds.
pub(super) fn act(
    cx: &mut TestAppContext,
    deck: &Entity<Deck>,
    (id, values): (&str, &[&str]),
    done: impl Fn(&Objects) -> bool,
) {
    update(cx, deck, |deck, cx| deck.object_command(id, values, cx));
    settle(cx, |app| {
        objects(app, deck).is_some_and(|objects| done(&objects))
    });
}

#[test]
fn shapes_take_place_look_and_lock_from_the_panel() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let rectangle = ("draw.shape", &["rectangle", "true"][..]);
    act(&mut cx, &deck, rectangle, |o| {
        o.selection.as_ref().is_some_and(|s| s.kind == "shape")
    });
    let x = crate::object::twips(3000).to_string();
    act(&mut cx, &deck, ("object.x", &[&x]), |o| {
        o.selection
            .as_ref()
            .is_some_and(|s| (s.rect.0 - 3000).abs() <= 1)
    });
    act(&mut cx, &deck, ("object.fill_none", &["0"]), |o| {
        o.selection.as_ref().is_some_and(|s| s.fill == Paint::None)
    });
    act(&mut cx, &deck, ("object.fill", &["255"]), |o| {
        o.selection
            .as_ref()
            .is_some_and(|s| s.fill == Paint::Color(255))
    });
    act(&mut cx, &deck, ("object.lock", &["true", "true"]), |o| {
        o.selection
            .as_ref()
            .is_some_and(|s| s.lock_position == Some(true))
    });
    let center = cx
        .update(|app| selection(app, &deck))
        .expect("a selection")
        .rotation_center;
    let twips = crate::object::twips;
    let (x, y) = (twips(center.0).to_string(), twips(center.1).to_string());
    act(&mut cx, &deck, ("object.lock", &["false", "false"]), |o| {
        o.selection
            .as_ref()
            .is_some_and(|s| s.lock_size == Some(false))
    });
    act(&mut cx, &deck, ("object.rotate", &["9000", &x, &y]), |o| {
        o.selection.as_ref().is_some_and(|s| s.rotation == 9000)
    });
    // A panel edit repaints the slide's picture.
    pictures_match(&mut cx, &deck);
}

#[test]
fn two_objects_group_align_and_ungroup() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let count = |n: usize| move |o: &Objects| o.objects.len() == n;
    act(
        &mut cx,
        &deck,
        ("draw.shape", &["rectangle", "true"]),
        count(3),
    );
    let roles = cx.update(|app| {
        let read = objects(app, &deck).expect("a read");
        read.objects
            .iter()
            .map(|o| o.placeholder.clone())
            .collect::<Vec<_>>()
    });
    assert_eq!(roles, ["title", "body", ""], "placeholders name their role");
    act(
        &mut cx,
        &deck,
        ("draw.shape", &["ellipse", "true"]),
        count(4),
    );
    update(&mut cx, &deck, |deck, cx| {
        deck.select_objects(&["2".into(), "3".into()], cx)
    });
    settle(&cx, |app| {
        selection(app, &deck).is_some_and(|s| s.count == 2 && s.can_group)
    });
    act(&mut cx, &deck, ("object.align_left", &[]), |o| {
        o.objects[2].rect.0 == o.objects[3].rect.0
    });
    act(&mut cx, &deck, ("object.group", &[]), |o| {
        o.selection
            .as_ref()
            .is_some_and(|s| s.kind == "group" && s.can_ungroup)
    });
    act(&mut cx, &deck, ("object.ungroup", &[]), count(4));
}

#[test]
fn guides_add_move_and_go() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    act(&mut cx, &deck, ("guide.add", &["1", "5000", "0"]), |o| {
        o.guides.len() == 1 && o.guides[0].kind == GuideKind::Vertical
    });
    act(&mut cx, &deck, ("guide.move", &["0", "7000", "0"]), |o| {
        o.guides.first().is_some_and(|guide| guide.at.0 == 7000)
    });
    // Snapping starts on; the command flips it.
    let snapped = cx.update(|app| objects(app, &deck).expect("a read").snap_guides);
    act(&mut cx, &deck, ("view.snap_guides", &[]), move |o| o.snap_guides != snapped);
    act(&mut cx, &deck, ("guide.delete", &["0", "true"]), |o| {
        o.guides.is_empty()
    });
}

#[test]
fn autofit_moves_between_its_three_rules() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let fit = |rule: &'static str| {
        move |o: &Objects| o.selection.as_ref().is_some_and(|s| s.autofit == rule)
    };
    act(&mut cx, &deck, ("draw.text", &["true"]), |o| {
        o.selection.as_ref().is_some_and(|s| s.kind == "text")
    });
    let now = cx
        .update(|app| selection(app, &deck))
        .expect("a box")
        .autofit;
    assert_eq!(now, "grow", "a new text box grows with its text");
    act(&mut cx, &deck, ("text.fit", &["off"]), fit("off"));
    act(&mut cx, &deck, ("text.fit", &["shrink"]), fit("shrink"));
    act(&mut cx, &deck, ("text.fit", &["grow"]), fit("grow"));
}

#[test]
fn a_repeated_commit_sends_once() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    act(
        &mut cx,
        &deck,
        ("draw.shape", &["rectangle", "true"]),
        |o| o.selection.is_some(),
    );
    let before = undo_steps(&mut cx, &deck);
    let (panel, now) = cx.update(|app| {
        let deck = deck.read(app);
        let rect = deck.active_tab().and_then(|tab| tab.extra.objects.clone());
        (
            deck.panel.clone(),
            rect.and_then(|o| o.selection).expect("a selection").rect.0,
        )
    });
    // Enter, then the blur that follows it.
    for _ in 0..2 {
        cx.update(|app| {
            crate::object::commit(
                &panel,
                "x",
                (now + 1000, Some(now)),
                move |deck, cx| {
                    let x = crate::object::twips(now + 1000).to_string();
                    deck.object_command("object.x", &[&x], cx)
                },
                app,
            )
        });
    }
    settle(&cx, |app| {
        selection(app, &deck).is_some_and(|s| (s.rect.0 - now - 1000).abs() <= 1)
    });
    assert_eq!(undo_steps(&mut cx, &deck), before + 1);
}

#[test]
fn pictures_insert_replace_and_crop() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let png = |name: &str, rgba| {
        let path = dir.path().join(name);
        image::RgbaImage::from_pixel(300, 200, image::Rgba(rgba))
            .save(&path)
            .expect("the png writes");
        ely_lok::file_url(&path)
    };
    let (red, blue) = (
        png("red.png", [255, 0, 0, 255]),
        png("blue.png", [0, 0, 255, 255]),
    );
    let picture = |o: &Objects| o.selection.as_ref().is_some_and(|s| s.kind == "image");
    act(&mut cx, &deck, ("image.insert", &[&red]), picture);
    let steps = undo_steps(&mut cx, &deck);
    act(&mut cx, &deck, ("image.replace", &[&blue]), |o| {
        o.objects.len() == 3
    });
    settle(&cx, |app| counts(app, &deck) == (1, 1));
    assert_eq!(
        undo_steps(&mut cx, &deck),
        steps + 1,
        "a replace is one step"
    );
    // The worker confirms crop mode from the kernel's frame.
    cx.update(|app| {
        let deck = deck.read(app);
        let doc = deck.active_tab().expect("a tab").doc.clone();
        let mut worker = deck.core.worker.lock().expect("the worker lock");
        worker.run(&doc, "image.crop", &[]).expect("crop mode");
    });
}

#[test]
fn a_text_box_takes_character_formats() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    act(&mut cx, &deck, ("draw.text", &["true"]), |o| {
        o.selection.as_ref().is_some_and(|s| s.kind == "text")
    });
    let state = |uno: &'static str, want: &'static str| {
        let deck = deck.clone();
        move |app: &gpui::App| {
            deck.read(app)
                .active_tab()
                .is_some_and(|tab| tab.view.read(app).state(uno) == Some(want))
        }
    };
    update(&mut cx, &deck, |deck, cx| {
        deck.object_command("format.bold", &[], cx)
    });
    settle(&cx, state(".uno:Bold", "true"));
    update(&mut cx, &deck, |deck, cx| {
        deck.object_command("format.size", &["28"], cx)
    });
    settle(&cx, state(".uno:FontHeight", "28"));
    update(&mut cx, &deck, |deck, cx| {
        deck.object_command("list.bullets", &[], cx)
    });
    settle(&cx, state(".uno:DefaultBullet", "true"));
}
