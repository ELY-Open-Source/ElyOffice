//! PDF, pictures and handouts from the deck (C10.6).

use std::path::Path;

use ely_office::prelude::*;
use gpui::{AppContext as _, Entity, TestAppContext};

use super::objects::blank;
use super::play::three_slides;
use super::{settle, undo_steps, update};
use crate::deck::Deck;
use crate::dialog::Dialog;
use crate::output::{Kind, Output, Range};

fn output(cx: &mut TestAppContext, deck: &Entity<Deck>, kind: Kind, range: Range) -> Output {
    cx.update(|app| {
        let tab = deck.read(app).active_tab().expect("a tab");
        Output {
            tab: tab.id,
            print: false,
            kind,
            range,
            per: 2,
            current: Some(1),
            hidden: tab.extra.slides.list.iter().map(|slide| !slide.visible).collect(),
        }
    })
}

/// Pages of a PDF, by pdftotext's page breaks.
fn pages(pdf: &Path) -> Option<usize> {
    let out = std::process::Command::new("pdftotext").arg(pdf).arg("-").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    Some(text.split('\x0c').count() - 1)
}

#[test]
fn slides_and_handouts_go_out_and_the_deck_stays_put() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    let steps = undo_steps(&mut cx, &deck);
    let (slides, handouts) = (dir.path().join("slides.pdf"), dir.path().join("handouts.pdf"));
    let asked = output(&mut cx, &deck, Kind::Slides, Range::All);
    update(&mut cx, &deck, |deck, cx| deck.export_pdf(&asked, slides.clone(), cx));
    let asked = output(&mut cx, &deck, Kind::Handouts, Range::Slides(1, 2));
    update(&mut cx, &deck, |deck, cx| deck.export_pdf(&asked, handouts.clone(), cx));
    settle(&cx, |_| slides.exists() && handouts.exists());
    assert_eq!(undo_steps(&mut cx, &deck), steps, "the deck is not touched");
    let (Some(slides), Some(handouts)) = (pages(&slides), pages(&handouts)) else {
        eprintln!("SKIP: no pdftotext to count pages");
        return;
    };
    assert_eq!(slides, 2, "the hidden slide stays out");
    assert_eq!(handouts, 1, "two slides on one handout page");
}

#[test]
fn pictures_are_one_png_a_shown_slide_and_never_overwrite() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    let folder = dir.path().join("out");
    std::fs::create_dir_all(&folder).expect("a folder");
    let asked = output(&mut cx, &deck, Kind::Pictures, Range::All);
    let into = (folder.clone(), "deck".to_string());
    update(&mut cx, &deck, |deck, cx| deck.write_pictures(&asked, into, cx));
    let written = |folder: &Path| {
        let mut names: Vec<String> = std::fs::read_dir(folder)
            .expect("reads")
            .map(|entry| entry.expect("an entry").file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    };
    settle(&cx, |_| written(&folder).len() == 2);
    assert_eq!(written(&folder), ["deck-1.png", "deck-2.png"], "slide three is hidden");
    let picture = image::open(folder.join("deck-1.png")).expect("a PNG");
    assert_eq!(picture.width(), 1920);
    let before = std::fs::read(folder.join("deck-1.png")).expect("reads");
    let asked = output(&mut cx, &deck, Kind::Pictures, Range::Current);
    let into = (folder.clone(), "deck".to_string());
    update(&mut cx, &deck, |deck, cx| deck.write_pictures(&asked, into, cx));
    for _ in 0..50 {
        cx.run_until_parked();
    }
    assert_eq!(std::fs::read(folder.join("deck-1.png")).expect("reads"), before, "never overwritten");
}

#[test]
fn the_check_lists_font_link_and_object_with_their_slides() {
    use crate::check::Problem;
    use crate::dialog::Dialog;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = super::deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/show/trouble.fodp");
    let path = dir.path().join("trouble.fodp");
    std::fs::copy(&fixture, &path).expect("copies");
    update(&mut cx, &deck, |deck, cx| deck.open_file(path.clone(), cx));
    settle(&cx, |app| {
        let tab = deck.read(app).active_tab();
        tab.is_some_and(|tab| !tab.view.read(app).missing_fonts().is_empty())
    });
    update(&mut cx, &deck, |deck, cx| deck.check_show(cx));
    settle(&cx, |app| matches!(deck.read(app).dialog, Some(Dialog::Check(_))));
    let found = cx.update(|app| match &deck.read(app).dialog {
        Some(Dialog::Check(found)) => found.clone(),
        _ => unreachable!("settled above"),
    });
    let short: Vec<(Problem, &str, Vec<usize>)> =
        found.iter().map(|f| (f.problem, f.name.as_str(), f.slides.clone())).collect();
    assert!(short.contains(&(Problem::Font, "Nowhere Sans", vec![0])), "{short:?}");
    let lost = found.iter().find(|f| f.name == "Nowhere Sans").expect("the font finding");
    assert!(!lost.instead.is_empty(), "the check names what drew it");
    assert!(short.contains(&(Problem::Link, "file:///nowhere/gone.png", vec![1])), "{short:?}");
    assert!(short.contains(&(Problem::Object, "Empty", vec![2])), "{short:?}");
}

#[cfg(unix)]
#[test]
fn pictures_claim_every_name_first_and_follow_no_link() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    let folder = dir.path().join("out");
    std::fs::create_dir_all(&folder).expect("a folder");
    // The second name links to a file not there.
    std::os::unix::fs::symlink("../unrelated.png", folder.join("deck-2.png")).expect("links");
    let asked = output(&mut cx, &deck, Kind::Pictures, Range::All);
    let into = (folder.clone(), "deck".to_string());
    update(&mut cx, &deck, |deck, cx| deck.write_pictures(&asked, into, cx));
    for _ in 0..200 {
        cx.run_until_parked();
    }
    assert!(!dir.path().join("unrelated.png").exists(), "the link is not followed");
    assert!(!folder.join("deck-1.png").exists(), "nothing of this run stays");
}

#[test]
fn master_view_exports_slides_and_has_no_current() {
    use crate::dialog::Dialog;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    update(&mut cx, &deck, |deck, cx| deck.masters(true, cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.masters));
    let window = cx.update(|app| deck.read(app).core.window.expect("a window"));
    cx.update_window(window, |_, window, app| {
        deck.update(app, |deck, cx| deck.on_export(&ely_ui::commands::Export, window, cx))
    })
    .expect("the window lives");
    settle(&cx, |app| matches!(deck.read(app).dialog, Some(Dialog::Output(_))));
    let (current, hidden) = cx.update(|app| match &deck.read(app).dialog {
        Some(Dialog::Output(output)) => (output.current, output.hidden.clone()),
        _ => unreachable!("settled above"),
    });
    assert_eq!((current, hidden), (None, vec![false, false, true]));
}

#[test]
fn a_late_check_never_covers_a_newer_dialog() {
    use crate::dialog::Dialog;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    // The export dialog is open before the check comes back.
    let open = output(&mut cx, &deck, Kind::Slides, Range::All);
    update(&mut cx, &deck, |deck, cx| {
        deck.check_show(cx);
        deck.dialog = Some(Dialog::Output(open));
    });
    for _ in 0..200 {
        cx.run_until_parked();
    }
    assert!(cx.update(|app| matches!(deck.read(app).dialog, Some(Dialog::Output(_)))));
}

#[test]
fn a_late_check_stays_away_after_another_dialog_closes() {
    use crate::dialog::Dialog;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let open = output(&mut cx, &deck, Kind::Slides, Range::All);
    update(&mut cx, &deck, |deck, cx| {
        deck.check_show(cx);
        deck.dialog = Some(Dialog::Output(open));
        cx.notify();
    });
    // The other dialog is cancelled before the check returns.
    update(&mut cx, &deck, |deck, cx| {
        deck.dialog = None;
        cx.notify();
    });
    for _ in 0..200 {
        cx.run_until_parked();
    }
    assert!(cx.update(|app| deck.read(app).dialog.is_none()), "the old check stays away");
}

#[test]
fn pictures_follow_the_slides_as_written_not_as_asked() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    // Asked while slide three hid; shown again before writing.
    let asked = output(&mut cx, &deck, Kind::Pictures, Range::All);
    assert_eq!(asked.hidden, [false, false, true]);
    update(&mut cx, &deck, |deck, cx| deck.design_command("slide.show", &[], cx));
    settle(&cx, |app| {
        let tab = deck.read(app).active_tab();
        tab.is_some_and(|tab| tab.extra.slides.list.iter().all(|slide| slide.visible))
    });
    let folder = dir.path().join("out");
    std::fs::create_dir_all(&folder).expect("a folder");
    let into = (folder.clone(), "deck".to_string());
    update(&mut cx, &deck, |deck, cx| deck.write_pictures(&asked, into, cx));
    let count = |folder: &Path| std::fs::read_dir(folder).expect("reads").count();
    settle(&cx, |_| count(&folder) == 3);
    for n in 1..=3 {
        let picture = image::open(folder.join(format!("deck-{n}.png"))).expect("a readable PNG");
        assert_eq!(picture.width(), 1920);
    }
}

#[test]
fn the_current_slide_is_the_one_just_picked() {
    // Seed 2 once read the slide before the turn landed.
    for seed in 0..4 {
        if !current_after_a_turn(seed) {
            eprintln!("SKIP: no LOK build");
            return;
        }
    }
}

/// False when there is no kernel to ask.
fn current_after_a_turn(seed: u64) -> bool {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::build(gpui::TestDispatcher::new(seed), None);
    let Some(deck) = blank(&mut cx, dir.path()) else {
        return false;
    };
    three_slides(&mut cx, &deck);
    update(&mut cx, &deck, |deck, cx| {
        let view = deck.active_tab().expect("a tab").view.clone();
        view.update(cx, |view, _| view.show_part(0));
    });
    settle(&cx, |app| {
        deck.read(app).active_tab().is_some_and(|tab| tab.extra.slides.current == 0 && tab.view.read(app).settled() && !tab.extra.reading)
    });
    // The turn and the ask go in one update.
    let window = cx.update(|app| deck.read(app).core.window.expect("a window"));
    cx.update_window(window, |_, window, app| {
        deck.update(app, |deck, cx| {
            let view = deck.active_tab().expect("a tab").view.clone();
            view.update(cx, |view, _| view.show_part(1));
            deck.on_export(&ely_ui::commands::Export, window, cx);
        })
    })
    .expect("the window lives");
    settle(&cx, |app| matches!(deck.read(app).dialog, Some(Dialog::Output(_))));
    let mut asked = cx.update(|app| match &deck.read(app).dialog {
        Some(Dialog::Output(output)) => output.clone(),
        _ => unreachable!("the output dialog"),
    });
    assert_eq!(asked.current, Some(2), "seed {seed}: the slide just picked is current");
    update(&mut cx, &deck, |deck, _| deck.dialog = None);
    (asked.range, asked.kind) = (Range::Current, Kind::Pictures);
    let folder = dir.path().join("out");
    std::fs::create_dir_all(&folder).expect("a folder");
    update(&mut cx, &deck, |deck, cx| deck.write_pictures(&asked, (folder.clone(), "deck".to_string()), cx));
    let names = |folder: &Path| -> Vec<String> {
        std::fs::read_dir(folder).expect("reads").map(|entry| entry.expect("an entry").file_name().to_string_lossy().into_owned()).collect()
    };
    settle(&cx, |_| !names(&folder).is_empty());
    assert_eq!(names(&folder), ["deck-2.png"], "seed {seed}: the current slide went out");
    true
}

#[test]
fn a_check_asked_after_a_waiting_output_wins() {
    // Seed 11 once let the waiting export win.
    for seed in 0..16 {
        let dir = tempfile::tempdir().expect("a temp dir");
        let mut cx = TestAppContext::build(gpui::TestDispatcher::new(seed), None);
        let Some(deck) = blank(&mut cx, dir.path()) else {
            eprintln!("SKIP: no LOK build");
            return;
        };
        three_slides(&mut cx, &deck);
        super::quiet(&cx, &deck);
        let window = cx.update(|app| deck.read(app).core.window.expect("a window"));
        cx.update_window(window, |_, window, app| {
            deck.update(app, |deck, cx| {
                let view = deck.active_tab().expect("a tab").view.clone();
                view.update(cx, |view, _| view.show_part(1));
                deck.on_export(&ely_ui::commands::Export, window, cx);
                deck.check_show(cx);
            })
        })
        .expect("the window lives");
        settle(&cx, |app| deck.read(app).dialog.is_some());
        super::quiet(&cx, &deck);
        cx.run_until_parked();
        let check = cx.update(|app| matches!(deck.read(app).dialog, Some(Dialog::Check(_))));
        assert!(check, "seed {seed}: the export asked first took the place of the check");
    }
}
