//! §20: a turned page shows whole, the next one ready.

use std::collections::HashSet;

use gpui::{Entity, TestAppContext, VisualTestContext};

use super::{file, settle, shown, windowed};
use crate::view::PdfView;

const PAGE: (f32, f32, &[&str]) = (612.0, 792.0, &["a page of text"]);

/// Every tile `screen` needs is painted.
fn whole(view: &PdfView, screen: &[crate::backdrop::Showing]) -> bool {
    !screen.is_empty() && screen.iter().all(|s| s.keys.iter().all(|(key, _)| view.tiles.contains_key(key)))
}

fn opened<'a>(cx: &'a mut TestAppContext, dir: &std::path::Path) -> (&'a mut VisualTestContext, Entity<PdfView>) {
    let path = file(dir, "four.pdf", &[PAGE, PAGE, PAGE, PAGE]);
    let (viewer, cx) = windowed(cx, dir, &[path], true);
    let view = shown(cx, &viewer);
    (cx, view)
}

#[test]
fn the_next_page_is_painted_before_the_turn_and_shows_whole() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (cx, view) = opened(&mut cx, dir.path());
    let sf = cx.update(|window, _| window.scale_factor());
    settle(cx, |app| {
        let view = view.read(app);
        !view.painting && whole(view, &view.showing(sf)) && whole(view, &view.ahead(sf))
    });
    cx.update(|_, app| view.update(app, |view, cx| view.turn(1, cx)));
    cx.update(|window, _| window.refresh());
    let (current, landed) = cx.update(|_, app| {
        let view = view.read(app);
        (view.current, whole(view, &view.showing(sf)))
    });
    assert_eq!(current, 1, "the turn lands on the next page");
    assert!(landed, "its first frame has every tile it shows");
    settle(cx, |app| {
        let view = view.read(app);
        !view.painting && whole(view, &view.ahead(sf))
    });
    let next = cx.update(|_, app| view.read(app).ahead(sf).iter().map(|s| s.place.page).collect::<Vec<_>>());
    assert!(next.contains(&2), "then the one after: {next:?}");
}

#[test]
fn a_new_tile_waits_for_the_rest_of_its_page() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = file(dir.path(), "long.pdf", &[(612.0, 2000.0, &["a long page"]), PAGE]);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    let sf = cx.update(|window, _| window.scale_factor());
    settle(cx, |app| {
        let view = view.read(app);
        !view.painting && whole(view, &view.showing(sf))
    });
    let before = cx.update(|_, app| drawn(view.read(app), sf, 0));
    cx.update(|_, app| {
        view.update(app, |view, _| {
            let missing = |view: &PdfView| -> Vec<crate::pages::TileKey> {
                let screen = view.showing(sf);
                screen.iter().flat_map(|s| s.keys.iter().map(|(key, _)| *key)).filter(|key| !view.tiles.contains_key(key)).collect()
            };
            let gap = (1..400).map(|step| step as f32 * 4.0).find_map(|down| {
                view.scroll.set_offset(gpui::point(gpui::px(0.0), gpui::px(-down)));
                Some(missing(view)).filter(|keys| keys.len() > 1)
            });
            let gap = gap.expect("a scroll reaches a row of several unpainted tiles");
            let early = crate::pages::picture((1, 1), vec![0; 4]).expect("a picture");
            view.tiles.insert(gap[0], early.clone());
            let now = drawn(view, sf, 0);
            assert!(!now.contains(&(std::sync::Arc::as_ptr(&early) as usize)), "one new tile alone is not drawn");
            assert_eq!(now, before, "the page shows what it showed");
        })
    });
}

/// Each piece `page` draws now, by image address.
fn drawn(view: &PdfView, sf: f32, page: usize) -> HashSet<usize> {
    let screen = view.showing(sf);
    let shown = screen.iter().find(|s| s.place.page == page).expect("the page shows");
    view.pieces(shown).iter().map(|(image, _)| std::sync::Arc::as_ptr(image) as usize).collect()
}

#[test]
fn scrolling_within_a_page_keeps_what_it_showed() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = file(dir.path(), "long.pdf", &[(612.0, 2000.0, &["a long page"]), PAGE]);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    let sf = cx.update(|window, _| window.scale_factor());
    let ready = |view: &Entity<PdfView>, app: &gpui::App| {
        let view = view.read(app);
        !view.painting && whole(view, &view.showing(sf))
    };
    settle(cx, |app| ready(&view, app));
    let before = cx.update(|_, app| drawn(view.read(app), sf, 0));
    let moved = cx.update(|_, app| {
        view.update(app, |view, _| {
            (1..400).map(|step| step as f32 * 4.0).find(|down| {
                view.scroll.set_offset(gpui::point(gpui::px(0.0), gpui::px(-down)));
                !whole(view, &view.showing(sf))
            })
        })
    });
    assert!(moved.is_some(), "a scroll reaches an unpainted row");
    assert_eq!(cx.update(|_, app| drawn(view.read(app), sf, 0)), before, "the page keeps what it showed");
    settle(cx, |app| ready(&view, app));
    let after = cx.update(|_, app| drawn(view.read(app), sf, 0));
    assert!(after != before && !after.is_empty(), "then shows the new rows whole");
}

#[test]
fn a_turned_page_lands_on_its_painted_screen_at_every_rotation() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (cx, view) = opened(&mut cx, dir.path());
    let sf = cx.update(|window, _| window.scale_factor());
    for rotate in [0, 90, 180, 270] {
        cx.update(|_, app| {
            view.update(app, |view, cx| {
                view.change(move |view, _| (view.mode, view.rotate) = (crate::layout::Mode::Single, rotate), cx);
                view.show(0, 0.0, cx);
            })
        });
        settle(cx, |app| {
            let view = view.read(app);
            !view.painting && view.go_to.is_none() && whole(view, &view.showing(sf)) && whole(view, &view.ahead(sf))
        });
        cx.update(|_, app| view.update(app, |view, cx| view.turn(1, cx)));
        cx.update(|window, _| window.refresh());
        let landed = cx.update(|_, app| whole(view.read(app), &view.read(app).showing(sf)));
        assert!(landed, "at {rotate}° the turn lands on the painted screen");
    }
}

/// A long first page shown whole; no tile asked after.
fn frozen<'a>(cx: &'a mut TestAppContext, dir: &std::path::Path) -> (&'a mut VisualTestContext, Entity<PdfView>, f32, HashSet<usize>) {
    let path = file(dir, "long.pdf", &[(612.0, 2000.0, &["a long page"]), (612.0, 2000.0, &["another"])]);
    let (viewer, cx) = windowed(cx, dir, &[path], true);
    let view = shown(cx, &viewer);
    let sf = cx.update(|window, _| window.scale_factor());
    settle(cx, |app| {
        let view = view.read(app);
        !view.painting && whole(view, &view.showing(sf))
    });
    let before = cx.update(|_, app| drawn(view.read(app), sf, 0));
    // Nothing more is asked of the worker.
    cx.update(|_, app| view.update(app, |view, _| view.painting = true));
    (cx, view, sf, before)
}

/// One new tile lands; the page must still show `before`.
fn one_lands(cx: &mut VisualTestContext, view: &Entity<PdfView>, sf: f32, before: &HashSet<usize>, after: &str) {
    cx.update(|_, app| {
        view.update(app, |view, _| {
            let screen = view.showing(sf);
            let key = screen.iter().find(|s| s.place.page == 0).expect("page 1 shows").keys[0].0;
            let early = crate::pages::picture((1, 1), vec![0; 4]).expect("a picture");
            view.tiles.insert(key, early.clone());
            let now = drawn(view, sf, 0);
            assert!(!now.contains(&(std::sync::Arc::as_ptr(&early) as usize)), "after {after}, one new tile alone is not drawn");
            if !before.is_empty() {
                assert_eq!(&now, before, "after {after}, the page shows its last whole draw");
            }
        })
    });
}

#[test]
fn after_an_edit_one_new_tile_waits_for_the_rest() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (cx, view, sf, before) = frozen(&mut cx, dir.path());
    let note = ely_pdf_engine::NewAnnot {
        page: 0,
        shape: ely_pdf_engine::Shape::Note((100.0, 500.0)),
        rgb: None,
        contents: "Look".into(),
        author: String::new(),
    };
    cx.update(|_, app| view.update(app, |view, cx| view.annotate("pdf.doing.add_note", vec![note], cx)));
    settle(cx, |app| view.read(app).edited == 1);
    one_lands(cx, &view, sf, &before, "an edit");
}

#[test]
fn after_a_page_move_the_old_page_never_stands_in() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (cx, view, sf, _) = frozen(&mut cx, dir.path());
    let doc = cx.update(|_, app| view.read(app).doc);
    cx.update(|_, app| view.update(app, |view, cx| view.edit_pages("pdf.doing.move_pages", move |worker, _| worker.move_pages(doc, &[1], 0), cx)));
    settle(cx, |app| view.read(app).edited == 1);
    assert!(cx.update(|_, app| drawn(view.read(app), sf, 0)).is_empty(), "another page's draw never shows");
    one_lands(cx, &view, sf, &HashSet::new(), "a move");
}

#[test]
fn a_tile_that_left_unshown_does_not_show_on_return() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = file(dir.path(), "long.pdf", &[(612.0, 2000.0, &["a long page"]), PAGE]);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    let sf = cx.update(|window, _| window.scale_factor());
    settle(cx, |app| {
        let view = view.read(app);
        !view.painting && whole(view, &view.showing(sf))
    });
    let before = cx.update(|_, app| drawn(view.read(app), sf, 0));
    cx.update(|_, app| {
        view.update(app, |view, _| {
            let missing = |view: &PdfView| -> Vec<crate::pages::TileKey> {
                view.showing(sf).iter().flat_map(|s| s.keys.iter().map(|(key, _)| *key)).filter(|key| !view.tiles.contains_key(key)).collect()
            };
            let down = (1..400).map(|step| step as f32 * 4.0).find(|down| {
                view.scroll.set_offset(gpui::point(gpui::px(0.0), gpui::px(-down)));
                missing(view).len() > 1
            });
            let down = down.expect("a row of several unpainted tiles");
            let early = crate::pages::picture((1, 1), vec![0; 4]).expect("a picture");
            view.tiles.insert(missing(view)[0], early);
            let frame = |view: &mut PdfView| {
                let (screen, ahead) = (view.showing(sf), view.ahead(sf));
                view.settle(&screen, &ahead);
            };
            frame(view);
            // Away past the page's end, then back.
            view.scroll.set_offset(gpui::point(gpui::px(0.0), gpui::px(-1900.0)));
            frame(view);
            view.scroll.set_offset(gpui::point(gpui::px(0.0), gpui::px(-down)));
            assert_eq!(drawn(view, sf, 0), before, "only the last whole draw shows on return");
        })
    });
}
