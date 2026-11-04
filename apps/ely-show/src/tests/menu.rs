use ely_gpui_component::theme::Theme;
use ely_lok::show::Objects;
use ely_office::prelude::*;
use gpui::{Entity, Modifiers, MouseButton, Pixels, Point, TestAppContext, VisualTestContext};

use super::objects::{act, blank, objects};
use super::settle;
use crate::deck::Deck;
use crate::menu::{Kind, Pick};

/// The canvas with the menu's host over it.
struct Host(Entity<Deck>);

impl gpui::Render for Host {
    fn render(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};
        let body = self.0.update(cx, |deck, cx| deck.body(window, cx));
        let menu = self.0.update(cx, |deck, cx| deck.context_menu(cx));
        gpui::div().relative().size_full().flex().flex_col().child(body).children(menu)
    }
}

fn frames(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
}

/// A slide point, in 1/100 mm, in window pixels.
fn spot(cx: &mut VisualTestContext, deck: &Entity<Deck>, (x, y): (i64, i64)) -> Point<Pixels> {
    frames(cx);
    cx.update(|_, app| {
        let read = deck.read(app).active_tab().expect("a tab").view.read(app);
        let twips = |mm100| crate::object::twips(mm100) as i32;
        read.origin() + read.place((twips(x), twips(y)))
    })
}

/// A point `1/part` into the first object of `kind`.
fn within(cx: &mut VisualTestContext, deck: &Entity<Deck>, kind: &str, part: i64) -> Point<Pixels> {
    let (x, y, w, h) = cx.update(|_, app| {
        let objects = objects(app, deck).expect("objects");
        objects.objects.iter().find(|object| object.kind == kind).expect("the object").rect
    });
    spot(cx, deck, (x + w / part, y + h / part))
}

fn click(cx: &mut VisualTestContext, at: Point<Pixels>, button: MouseButton) {
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, button, Modifiers::none());
    cx.simulate_mouse_up(at, button, Modifiers::none());
}

/// Right-clicks and waits for a menu of `kind`.
fn right_click(cx: &mut VisualTestContext, deck: &Entity<Deck>, at: Point<Pixels>, kind: Kind) {
    cx.update(|_, app| deck.update(app, |deck, _| deck.menu = None));
    click(cx, at, MouseButton::Right);
    for _ in 0..3000 {
        cx.run_until_parked();
        if cx.update(|_, app| deck.read(app).menu.as_ref().is_some_and(|offer| offer.what.kind == kind)) {
            return frames(cx);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let got = cx.update(|_, app| deck.read(app).menu.as_ref().map(|offer| offer.what.kind));
    panic!("no {kind:?} menu, got {got:?}");
}

fn on(cx: &mut VisualTestContext, deck: &Entity<Deck>, pick: Pick) -> bool {
    cx.update(|_, app| deck.read(app).menu.as_ref().expect("a menu").what.on(pick))
}

fn choose(cx: &mut VisualTestContext, deck: &Entity<Deck>, pick: Pick) {
    cx.update(|window, app| deck.update(app, |deck, cx| deck.choose(pick, window, cx)));
}

fn table_height(objects: &Objects) -> Option<i64> {
    objects.objects.iter().find(|object| object.kind == "table").map(|object| object.rect.3)
}

#[test]
fn right_clicks_open_powerpoint_rows_on_each_target() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(|cx| Theme::update(cx, |theme| theme.reduced_motion = true));
    settle(&cx, |app| objects(app, &deck).is_some_and(|o| !o.objects.is_empty()));
    let window = cx.add_window(|_, _| Host(deck.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    let corner = spot(&mut shown, &deck, (27_000, 15_000));
    right_click(&mut shown, &deck, corner, Kind::Slide);
    let clipboard = shown.debug_bounds("menu-button-0-0").expect("the clipboard row");
    assert!((clipboard.origin - corner).x < gpui::px(40.0), "at the click");
    assert!(on(&mut shown, &deck, Pick::Layout(0)) && on(&mut shown, &deck, Pick::Comment));
    // Typing in the title: list rows.
    let title = within(&mut shown, &deck, "text", 2);
    click(&mut shown, title, MouseButton::Left);
    right_click(&mut shown, &deck, title, Kind::Text);
    assert!(on(&mut shown, &deck, Pick::Bullets) && on(&mut shown, &deck, Pick::Numbering));
    let rectangle = ("draw.shape", &["rectangle", "true"][..]);
    act(&mut cx, &deck, rectangle, |o| o.selection.as_ref().is_some_and(|s| s.kind == "shape"));
    let shape = within(&mut shown, &deck, "shape", 2);
    right_click(&mut shown, &deck, shape, Kind::Object);
    assert!(on(&mut shown, &deck, Pick::Back) && on(&mut shown, &deck, Pick::Forward));
    assert!(!on(&mut shown, &deck, Pick::Group), "one shape does not group");
    choose(&mut shown, &deck, Pick::Back);
    settle(&cx, |app| objects(app, &deck).is_some_and(|o| o.objects.first().is_some_and(|first| first.kind == "shape")));
    let path = dir.path().join("red.png");
    image::RgbaImage::from_pixel(300, 200, image::Rgba([255, 0, 0, 255])).save(&path).expect("the png writes");
    let image = ("image.insert", &[ely_lok::file_url(&path)][..]);
    let image: (&str, Vec<&str>) = (image.0, image.1.iter().map(String::as_str).collect());
    act(&mut cx, &deck, (image.0, &image.1), |o| o.selection.as_ref().is_some_and(|s| s.kind == "image"));
    let picture = within(&mut shown, &deck, "image", 2);
    right_click(&mut shown, &deck, picture, Kind::Picture);
    assert!(on(&mut shown, &deck, Pick::Replace) && on(&mut shown, &deck, Pick::Crop));
    act(&mut cx, &deck, ("table.insert", &["2", "2"]), |o| table_height(o).is_some());
    // A quarter in: the middle is where the borders cross.
    let cell = within(&mut shown, &deck, "table", 4);
    right_click(&mut shown, &deck, cell, Kind::Table);
    assert!(on(&mut shown, &deck, Pick::RowAbove), "inserting needs no cell");
    // Deleting rows needs the cursor in a cell.
    click(&mut shown, cell, MouseButton::Left);
    right_click(&mut shown, &deck, cell, Kind::Table);
    assert!(on(&mut shown, &deck, Pick::DeleteRows) && on(&mut shown, &deck, Pick::Merge));
    let before = cx.update(|app| objects(app, &deck).and_then(|o| table_height(&o))).expect("a table");
    choose(&mut shown, &deck, Pick::RowBelow);
    settle(&cx, |app| objects(app, &deck).and_then(|o| table_height(&o)).is_some_and(|h| h > before));
    // A right click that ends an edit gets no menu.
    click(&mut shown, corner, MouseButton::Left);
    right_click(&mut shown, &deck, corner, Kind::Slide);
    choose(&mut shown, &deck, Pick::Layout(0));
    let layout = |app: &gpui::App| deck.read(app).active_tab().and_then(|tab| tab.extra.design.as_ref().map(|d| d.layout));
    settle(&cx, |app| layout(app) == Some(20));
}

/// Whether the Edit menu offers Undo and Redo now.
fn offered(app: &gpui::App) -> (bool, bool) {
    // Menus are first set once the kernel has spoken.
    let menus = app.get_menus().unwrap_or_default();
    let on = |name: &str| {
        menus.iter().flat_map(|menu| &menu.items).any(|item| {
            matches!(item, gpui::OwnedMenuItem::Action { name: shown, disabled: false, .. } if shown == name)
        })
    };
    (on("Undo"), on("Redo"))
}

#[test]
fn undo_and_redo_follow_the_deck_history() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    assert_eq!(cx.update(|app| offered(app)), (false, false), "an empty history");
    act(&mut cx, &deck, ("draw.shape", &["rectangle", "true"]), |o| {
        o.selection.as_ref().is_some_and(|s| s.kind == "shape")
    });
    settle(&cx, |app| offered(app) == (true, false));
    cx.update(|app| deck.update(app, |deck, cx| deck.run_active("edit.undo", &[], cx)));
    settle(&cx, |app| offered(app).1);
}

#[test]
fn closing_the_last_deck_greys_undo() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    act(&mut cx, &deck, ("draw.shape", &["rectangle", "true"]), |o| {
        o.selection.as_ref().is_some_and(|s| s.kind == "shape")
    });
    settle(&cx, |app| offered(app) == (true, false));
    cx.update(|app| {
        deck.update(app, |deck, cx| {
            let id = deck.core.active().expect("a deck");
            deck.close_tab(id, cx);
        })
    });
    settle(&cx, |app| offered(app) == (false, false));
    assert_eq!(cx.update(|app| offered(app)), (false, false), "no deck, nothing to undo");
}
