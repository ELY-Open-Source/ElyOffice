//! The status bar's Present button (PR #134).

use ely_gpui_component::theme::Theme;
use gpui::{Modifiers, TestAppContext, VisualTestContext};

use super::focus::{Front, frames};
use super::objects::blank;
use super::play::{place, three_slides};
use super::settle;
use crate::slideshow::steps::Place;

#[test]
fn present_plays_from_the_slide_on_screen() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    three_slides(&mut cx, &deck);
    cx.update(|cx| Theme::update(cx, |theme| theme.reduced_motion = true));
    let window = cx.add_window(|_, _| Front(deck.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    frames(&mut shown);
    let button = shown.debug_bounds("status-item show-present").expect("Present in the status bar");
    shown.simulate_click(button.center(), Modifiers::none());
    settle(&cx, |app| deck.read(app).show.is_some());
    let show = cx.update(|app| deck.read(app).show.clone().expect("playing"));
    assert_eq!(place(&mut cx, &show), Place::Slide { part: 2, clicks: 0 }, "from the shown slide");
}
