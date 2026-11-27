//! The live scoreboards over a running show.

use std::sync::Mutex;

use gpui::TestAppContext;

use super::objects::blank;
use super::play::{three_slides, typed};
use super::{settle, update};
use crate::keep::Held;
use crate::slideshow::scoreboard::{Look, Source, renamed, rows};

/// What the stand-in ElyTable shows; None is closed.
static POINTS: Mutex<Option<&str>> = Mutex::new(Some("1"));

fn table(words: &[&str]) -> std::io::Result<String> {
    let points = POINTS.lock().expect("the points").ok_or_else(|| std::io::Error::other("closed"))?;
    Ok(answer(words, points))
}

/// ElyTable's answer, its mark the points shown.
fn answer(words: &[&str], points: &str) -> String {
    match words {
        ["score.read", "7:0", _, seen] if *seen == points => {
            // ElyTable holds an unchanged read; this stands in.
            std::thread::sleep(std::time::Duration::from_millis(20));
            serde_json::json!({ "mark": points }).to_string()
        }
        ["score.read", "7:0", sheet] | ["score.read", "7:0", sheet, _] => {
            serde_json::json!({ "mark": points, "rows": [[sheet, points]] }).to_string()
        }
        _ => panic!("{words:?}"),
    }
}

/// ElyTable restarted: the same book has a new key.
fn restarted(words: &[&str]) -> std::io::Result<String> {
    let book = |key: &str, name: &str, path: Option<&str>| {
        serde_json::json!({ "book": key, "name": name, "path": path, "sheets": ["Final"] })
    };
    Ok(match words {
        ["docs"] => "[]".to_string(),
        ["score.list"] => serde_json::json!([
            book("9:0", "cup.ods", Some("/games/cup.ods")),
            book("9:1", "Untitled 1", None),
            book("9:2", "Untitled 2", None),
            book("9:3", "Untitled 2", None),
        ])
        .to_string(),
        ["score.read", key, _] if key.starts_with("9:") => serde_json::json!([[key, "4"]]).to_string(),
        _ => return Err(std::io::Error::other("that workbook is no longer open in ElyTable")),
    })
}

#[test]
fn a_kept_sheet_is_found_again_by_its_file_or_its_one_name() {
    let kept = |book: &str, path: Option<&str>, name: &str| Source {
        book: book.into(),
        path: path.map(str::to_owned),
        name: name.into(),
        sheet: "Final".into(),
    };
    let read = |source: Source| rows(&source, restarted).map(|rows| rows[0][0].clone());
    assert_eq!(read(kept("", Some("/games/cup.ods"), "old name")).expect("by file"), "9:0");
    assert_eq!(read(kept("7:0", None, "Untitled 1")).expect("by name, key gone"), "9:1");
    assert!(read(kept("", Some("/elsewhere/cup.ods"), "cup.ods")).is_err(), "another file is another book");
    let twice = read(kept("", None, "Untitled 2")).expect_err("two such books");
    assert!(twice.to_string().contains("open several times"), "{twice}");
    let gone = |_: &[&str]| Err::<String, _>(std::io::Error::from(std::io::ErrorKind::ConnectionRefused));
    let closed = rows(&kept("7:0", None, "Game"), gone).expect_err("no ElyTable");
    assert_eq!(closed.to_string(), "Game / Final is not open");
    // A save takes the linked book's present name and file.
    let held = vec![Held { at: 0, score: Some(kept("9:0", None, "Untitled 7")), timer: None }];
    let named = renamed(held, restarted).remove(0).score.expect("a score");
    assert_eq!((named.name.as_str(), named.path.as_deref()), ("cup.ods", Some("/games/cup.ods")));
}

fn source(sheet: &str) -> Option<Source> {
    Some(Source { book: "7:0".into(), path: None, name: "Game".into(), sheet: sheet.into() })
}

#[test]
fn each_slide_follows_its_own_sheet_and_keeps_scores_on_failure() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    update(&mut cx, &deck, |deck, cx| deck.start_show(false, cx));
    settle(&cx, |app| deck.read(app).show.is_some());
    let show = cx.update(|app| deck.read(app).show.clone().expect("playing"));
    let slides = vec![source("First"), source("Second"), None];
    cx.update(|app| show.update(app, |show, cx| show.follow_scores(slides, table, cx)));
    let reads = |cx: &TestAppContext, part: usize, sheet: &'static str, want: &'static str, failed: bool| {
        settle(cx, |app| {
            let scores = show.read(app).board.as_ref().and_then(|board| board.of(part));
            scores.is_some_and(|scores| scores.rows == [[sheet, want]] && scores.failed.is_some() == failed)
        });
    };
    reads(&cx, 0, "First", "1", false);
    reads(&cx, 1, "Second", "1", false);
    assert!(cx.update(|app| show.read(app).board.as_ref().expect("a board").of(2).is_none()), "no sheet, no board");
    *POINTS.lock().expect("the points") = Some("2");
    reads(&cx, 0, "First", "2", false);
    *POINTS.lock().expect("the points") = None;
    reads(&cx, 0, "First", "2", true);
    assert!(cx.update(|app| show.read(app).warning().is_some()), "the presenter is told");
    *POINTS.lock().expect("the points") = Some("3");
    reads(&cx, 0, "First", "3", false);
    let look = |cx: &mut TestAppContext| cx.update(|app| show.read(app).board.as_ref().expect("a board").look);
    for want in [Look::Corner, Look::Full, Look::Off, Look::Center] {
        typed(&mut cx, &show, "s");
        assert_eq!(look(&mut cx), want);
    }
    typed(&mut cx, &show, "escape");
    settle(&cx, |app| deck.read(app).show.is_none());
}

fn steady(words: &[&str]) -> std::io::Result<String> {
    Ok(answer(words, "9"))
}

#[test]
fn a_linked_slide_is_followed_while_editing_until_unlinked() {
    use ely_office::prelude::*;

    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    update(&mut cx, &deck, |deck, cx| deck.follow_slides(steady, cx));
    assert!(cx.update(|app| deck.read(app).board.is_none()), "nothing linked");
    let link = |cx: &mut TestAppContext, to: Option<Source>| {
        update(cx, &deck, |deck, cx| {
            let tab = deck.core.active().expect("a tab");
            let extra = &mut deck.tab_mut(tab).expect("the tab").extra;
            let name = extra.slides.list[extra.slides.current].name.clone();
            match to {
                Some(to) => extra.scoreboards.insert(name, to),
                None => extra.scoreboards.remove(&name),
            };
            deck.follow_slides(steady, cx);
        });
    };
    link(&mut cx, source("Live"));
    settle(&cx, |app| {
        let scores = deck.read(app).board.as_ref().and_then(|board| board.of(0));
        scores.is_some_and(|scores| scores.rows == [["Live", "9"]])
    });
    link(&mut cx, None);
    assert!(cx.update(|app| deck.read(app).board.is_none()), "unlinked");
}

#[test]
fn linking_empties_each_new_slide_and_keeps_later_additions() {
    use ely_office::prelude::*;

    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    let (tab, names) = cx.update(|app| {
        let deck = deck.read(app);
        let tab = deck.core.active().expect("a tab");
        let slides = &deck.tab(tab).expect("the tab").extra.slides.list;
        (tab, slides.iter().map(|slide| slide.name.clone()).collect::<Vec<_>>())
    });
    // The shown slide's object count, once read.
    let shows = |cx: &TestAppContext, part: usize, count: usize| {
        settle(cx, |app| {
            let extra = &deck.read(app).tab(tab).expect("the tab").extra;
            extra.slides.current == part && extra.objects.as_ref().is_some_and(|objects| objects.objects.len() == count)
        });
    };
    let turn = |cx: &mut TestAppContext, part: usize| {
        let view = cx.update(|app| deck.read(app).tab(tab).expect("the tab").view.clone());
        cx.update(|app| view.update(app, |view, _| view.show_part(part)));
    };
    turn(&mut cx, 0);
    settle(&cx, |app| {
        let extra = &deck.read(app).tab(tab).expect("the tab").extra;
        extra.slides.current == 0 && extra.objects.as_ref().is_some_and(|objects| !objects.objects.is_empty())
    });
    let all = names.clone();
    update(&mut cx, &deck, |deck, cx| deck.link_scores(tab, all, source("First").expect("a source"), steady, cx));
    settle(&cx, |app| deck.read(app).tab(tab).expect("the tab").extra.scoreboards.len() == 3);
    for part in [1, 2, 0] {
        turn(&mut cx, part);
        shows(&cx, part, 0);
    }
    // One undo brings all three back; redo empties them again.
    let history = |cx: &mut TestAppContext, id: &str| {
        let view = cx.update(|app| deck.read(app).tab(tab).expect("the tab").view.clone());
        cx.update(|app| view.update(app, |view, _| view.run(id, &[]))).expect("runs");
    };
    history(&mut cx, "edit.undo");
    for part in [1, 2, 0] {
        turn(&mut cx, part);
        settle(&cx, |app| {
            let extra = &deck.read(app).tab(tab).expect("the tab").extra;
            extra.slides.current == part && extra.objects.as_ref().is_some_and(|objects| !objects.objects.is_empty())
        });
    }
    history(&mut cx, "edit.redo");
    for part in [1, 2, 0] {
        turn(&mut cx, part);
        shows(&cx, part, 0);
    }
    update(&mut cx, &deck, |deck, cx| deck.object_command("draw.text", &["true"], cx));
    shows(&cx, 0, 1);
    let first = vec![names[0].clone()];
    update(&mut cx, &deck, |deck, cx| deck.link_scores(tab, first, source("Second").expect("a source"), steady, cx));
    settle(&cx, |app| {
        let links = &deck.read(app).tab(tab).expect("the tab").extra.scoreboards;
        links.get(&names[0]) == source("Second").as_ref()
    });
    shows(&cx, 0, 1);
}
