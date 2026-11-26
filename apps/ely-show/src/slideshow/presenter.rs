//! The presenter's screen: slide, next slide, notes, time.

use ely_gpui_component::{
    buttons::{Button, ToggleButton, ToggleItem},
    feedback::InlineMessage,
    primitives::Severity,
    primitives::FocusScope,
    theme::{ActiveTheme as _, ControlSize, TextSize},
};
use std::{cell::Cell, rc::Rc, time::Duration};

use gpui::{
    App, Bounds, Context, Entity, FocusHandle, InteractiveElement as _, IntoElement,
    ParentElement as _, Pixels, Render, StatefulInteractiveElement as _, Styled as _, Task, Window,
    canvas, div,
};

use super::{Show, stage};
use crate::deck::Deck;

pub struct Presenter {
    pub show: Entity<Show>,
    pub focus: FocusHandle,
    /// Where the current slide was last drawn.
    shown_at: Rc<Cell<Bounds<Pixels>>>,
    _tick: Task<()>,
}

impl Presenter {
    pub fn new(show: Entity<Show>, cx: &mut Context<Self>) -> Self {
        cx.observe(&show, |_, _, cx| cx.notify()).detach();
        // The clock moves once a second.
        let tick = cx.spawn(async |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        });
        Presenter {
            show,
            focus: cx.focus_handle(),
            shown_at: Rc::default(),
            _tick: tick,
        }
    }
}

/// `m:ss`, or `h:mm:ss` past an hour.
pub fn clock(seconds: u64) -> String {
    let (h, m, s) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    match h {
        0 => format!("{m}:{s:02}"),
        _ => format!("{h}:{m:02}:{s:02}"),
    }
}

fn button(
    show: &Entity<Show>,
    (id, label): (&'static str, gpui::SharedString),
    act: fn(&mut Show, &mut Context<Show>),
) -> Button {
    let show = show.clone();
    Button::new(id, label)
        .size(ControlSize::Sm)
        .on_click(move |_, _, cx| show.update(cx, act))
}

fn toggle(
    show: &Entity<Show>,
    (id, label, on): (&'static str, gpui::SharedString, bool),
    act: fn(&mut Show, &mut Context<Show>),
) -> ToggleButton {
    let show = show.clone();
    ToggleButton::new(id, ToggleItem::new(id).label(label), on)
        .size(ControlSize::Sm)
        .on_toggle(move |_, _, cx| show.update(cx, act))
}

impl Presenter {
    fn bar(&self, cx: &App) -> impl IntoElement {
        let show = self.show.read(cx);
        let label = |key: &str| Deck::text(cx, key, &[]);
        let place = match (show.part(), show.typed.is_empty()) {
            (_, false) => Deck::text(cx, "show.play.going", &[("number", &show.typed)]),
            (Some(part), true) => Deck::text(
                cx,
                "show.play.place",
                &[
                    ("number", &(part + 1).to_string()),
                    ("count", &show.slides.len().to_string()),
                ],
            ),
            (None, true) => label("show.play.ended"),
        };
        let place = div()
            .flex()
            .items_center()
            .gap_2()
            .child(place)
            .children(show.warning().map(|why| InlineMessage::new(Severity::Danger, why)));
        let now = jiff::Zoned::now().strftime("%H:%M").to_string();
        let row = || div().flex().items_center().gap_2();
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .child(
                row()
                    .child(button(
                        &self.show,
                        ("show-play-back", label("show.play.back")),
                        Show::previous,
                    ))
                    .child(button(
                        &self.show,
                        ("show-play-next", label("show.play.next")),
                        Show::next,
                    ))
                    .child(place),
            )
            .child(
                row()
                    .child(clock(show.began.elapsed().as_secs()))
                    .child(div().text_color(cx.theme().palette().fg_muted).child(now)),
            )
            .child(
                row()
                    .children(show.part().and_then(|part| show.countdown(part, cx)).into_iter().flat_map(|face| {
                        [
                            button(&self.show, ("show-play-timer", label(face.phase.run_key())), Show::toggle_timer),
                            button(&self.show, ("show-play-timer-reset", label("show.timer.reset")), Show::reset_timer),
                        ]
                    }))
                    .children(show.part().filter(|part| show.board.as_ref().is_some_and(|board| board.linked(*part))).and_then(|part| show.countdown(part, cx)).map(|face| button(&self.show, ("show-play-focus", label(face.focus_key())), Show::swap_focus)))
                    .children(show.board.is_some().then(|| button(&self.show, ("show-play-score", label("show.score.cycle")), Show::cycle_scores)))
                    .child(toggle(
                        &self.show,
                        ("show-play-black", label("show.play.black"), show.black),
                        Show::toggle_black,
                    ))
                    .child(toggle(
                        &self.show,
                        ("show-play-laser", label("show.play.laser"), show.laser),
                        Show::toggle_laser,
                    ))
                    .child(button(
                        &self.show,
                        ("show-play-end", label("show.play.end")),
                        Show::close,
                    )),
            )
    }
}

impl Render for Presenter {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.show.update(cx, |show, _| show.free(1, window));
        let show = self.show.read(cx);
        let room = window.viewport_size();
        if show.moving() {
            window.request_animation_frame();
        }
        let theme = cx.theme();
        let big = stage::fit(show.slide, (room.width * 0.6, room.height * 0.8));
        let small = stage::fit(show.slide, (room.width * 0.3, room.height * 0.35));
        let next = match show.upcoming() {
            Some(part) => stage::still(show, part, small),
            None => div().w(small.0).h(small.1).into_any_element(),
        };
        let notes = show.part().map(|part| show.slides[part].notes.clone());
        let (moved, keys) = (self.show.clone(), self.show.clone());
        let (seen, store) = (self.shown_at.clone(), self.shown_at.clone());
        let current = div()
            .id("show-presenter-slide")
            .relative()
            .on_mouse_move(move |event, _, cx| {
                let (at, bounds) = (event.position, seen.get());
                let x = (at.x - bounds.origin.x) / bounds.size.width;
                let y = (at.y - bounds.origin.y) / bounds.size.height;
                let inside = (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y);
                moved.update(cx, |show, cx| show.point(inside.then_some((x, y)), cx))
            })
            .child(stage::stage(show, &self.show, big, cx))
            .child(
                canvas(move |bounds, _, _| store.set(bounds), |_, _, _, _| {})
                    .absolute()
                    .size_full(),
            );
        let side = div()
            .flex()
            .flex_col()
            .gap_4()
            .flex_1()
            .min_h_0()
            .child(next)
            .child(
                div()
                    .id("show-notes")
                    .flex_1()
                    .overflow_y_scroll()
                    .text_size(theme.text_size(TextSize::Lg))
                    .children(notes),
            );
        // Keys reach ancestors of the focused scope.
        div()
            .size_full()
            .on_key_down(move |event, _, cx| {
                if keys.update(cx, |show, cx| show.key(&event.keystroke, cx)) {
                    cx.stop_propagation();
                }
            })
            .child(
                FocusScope::new(&self.focus)
                    .root()
                    .size_full()
                    .bg(theme.palette().surface)
                    .text_color(theme.palette().fg)
                    .font_family(theme.font_family.clone())
                    .child(
                        div()
                            .id("show-presenter")
                            .size_full()
                            .p_6()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .child(self.bar(cx))
                            .child(
                                div()
                                    .flex()
                                    .gap_6()
                                    .flex_1()
                                    .min_h_0()
                                    .child(current)
                                    .child(side),
                            ),
                    ),
            )
    }
}
