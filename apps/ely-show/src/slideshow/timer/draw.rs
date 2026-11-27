//! How a countdown looks over a slide.

use std::rc::Rc;

use ely_gpui_component::{
    buttons::IconButton,
    primitives::IconName,
    theme::{ActiveTheme as _, ControlSize},
};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, AppContext as _, Entity, FontWeight, InteractiveElement as _, IntoElement as _, MouseButton,
    ParentElement as _, Pixels, SharedString, StatefulInteractiveElement as _, Styled as _, Window,
    Rgba, canvas, div, px, relative, rgb,
};

use super::{Clocks, Face, Left, Phase, Spot, Timer, drag::{Drag, Ghost}, ending};

/// The share of a slide's height a countdown's band takes.
pub const BAND: f32 = 0.18;

/// The slide's share the unfocused scores keep.
pub const ASIDE: f32 = 0.38;

/// How the countdown shares its slide.
#[derive(Clone, Copy)]
pub enum Room {
    Alone,
    /// Centred scores have the focus.
    Beside,
    /// It has the focus; scores step aside below.
    Ahead,
}

// Fixed: no theme's warning colors carry white digits.
const ORANGE: u32 = 0xe05d00;
const RED: u32 = 0xd32626;

/// The room the buttons take beside the countdown.
const BUTTONS: Pixels = px(46.0);
/// Wide enough that the show's buttons stay hovered.
const WIDE: Pixels = px(260.0);

pub type Act = Rc<dyn Fn(&mut Window, &mut App)>;

/// What a click on a countdown reaches.
pub struct Hand {
    pub clocks: Entity<Clocks>,
    pub timer: Timer,
    /// The start or pause button's words, then reset's.
    pub run: SharedString,
    pub reset: SharedString,
    /// Gives the other one the focus, beside scores.
    pub swap: Option<(SharedString, Act)>,
    /// The editor's edit button; the show has none.
    pub edit: Option<(SharedString, Act)>,
}

/// The countdown over a slide; `beside` scores, it shrinks.
pub fn face(face: &Face, hand: Hand, (w, h): (Pixels, Pixels), room: Room, cx: &App) -> AnyElement {
    let palette = cx.theme().palette();
    // With the focus, it is large above the scores.
    let (spot, at) = match (room, face.spot) {
        (Room::Ahead, _) => (Spot::Center, None),
        // Beside centred scores, only a band stays free.
        (Room::Beside, Spot::Center) => (Spot::Top, face.at),
        (_, spot) => (spot, face.at),
    };
    let tall = match room {
        Room::Ahead => h * (1.0 - ASIDE),
        Room::Alone | Room::Beside => h,
    };
    let text = match spot {
        Spot::Center => (h * 0.28).min(w * 0.16),
        Spot::Top | Spot::Bottom | Spot::Corner => h * 0.1,
    };
    let alarm = |color: Rgba| (color.into(), palette.on_media);
    let (back, ink) = match (face.phase, face.left) {
        (Phase::Over, _) | (_, Left::Tenth) => alarm(rgb(RED)),
        (_, Left::Fifth) => alarm(rgb(ORANGE)),
        (Phase::Paused, Left::Plenty) => (palette.surface, palette.fg_muted),
        (Phase::Ready | Phase::Running, Left::Plenty) => (palette.surface, palette.fg),
    };
    let (clocks, timer) = (hand.clocks.clone(), hand.timer);
    // Blocking hides the group's hover, which the show needs.
    let editing = hand.edit.is_some();
    // The large, focused clock holds no dragged place.
    let movable = editing && !matches!(room, Room::Ahead);
    let drag = Drag::default();
    let drawn = drag.drawn.clone();
    let pill = div()
        .id("show-timer-clock")
        .when(movable, |pill| {
            pill.occlude().on_drag(drag, |drag, offset, _, cx| {
                drag.hold(offset);
                cx.new(|_| Ghost)
            })
        })
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(move |_, _, cx| clocks.update(cx, |clocks, cx| clocks.toggle(timer, cx)))
        .px(text * 0.5)
        .rounded(text * 0.25)
        .bg(back)
        .border_1()
        .border_color(palette.border)
        .shadow_lg()
        .text_color(ink)
        .text_size(text)
        .line_height(relative(1.35))
        .font_weight(FontWeight::BOLD)
        .whitespace_nowrap()
        .relative()
        .child(face.text.clone())
        .child(canvas(move |bounds, _, _| drawn.set(bounds), |_, _, _, _| {}).absolute().inset_0());
    // Buttons near an edge turn towards the slide.
    let (above, side) = match (at, spot) {
        (Some((x, y)), _) => (y > 0.75, if x > 0.7 { Side::Right } else if x < 0.3 { Side::Left } else { Side::Middle }),
        (None, Spot::Corner) => (true, Side::Right),
        (None, Spot::Bottom) => (true, Side::Middle),
        (None, Spot::Top | Spot::Center) => (false, Side::Middle),
    };
    let held = div().group("show-timer").relative().min_w(WIDE).flex();
    let held = match (at, spot) {
        (None, Spot::Corner) => held.justify_end().pt(BUTTONS),
        (None, Spot::Top) => held.justify_center().pb(BUTTONS),
        (None, Spot::Bottom) => held.justify_center().pt(BUTTONS),
        // Even padding keeps the clock's middle in place.
        (None, Spot::Center) | (Some(_), _) => held.justify_center().py(BUTTONS),
    };
    let held = held.child(pill).child(buttons(hand, face.phase, above, side, cx));
    let place = div().absolute().flex().justify_center();
    let placed = match (at, spot) {
        (Some((x, y)), _) => place.left(w * x - w).top(h * y - h / 2.0).w(w * 2.0).h(h).items_center(),
        (None, Spot::Top) => place.top(h * 0.022).left_0().w(w).items_start(),
        (None, Spot::Center) => place.top_0().left_0().w(w).h(tall).items_center(),
        (None, Spot::Bottom) => place.bottom(h * 0.03).left_0().w(w).items_end(),
        (None, Spot::Corner) => place.bottom(h * 0.03).right(w * 0.02),
    }
    .child(held);
    let party = face.party.then(|| ending::over(face.ending, (w, h), cx)).flatten();
    let Some(party) = party else {
        return placed.into_any_element();
    };
    // The effect lies under the clock.
    let whole = div().absolute().top_0().left_0().w(w).h(h);
    whole.child(party).child(placed).into_any_element()
}

/// Which of the clock's edges the buttons keep to.
#[derive(Clone, Copy)]
enum Side {
    Left,
    Middle,
    Right,
}

/// Icons in a floating capsule; hidden in shows until hovered.
fn buttons(hand: Hand, phase: Phase, above: bool, side: Side, cx: &App) -> AnyElement {
    let button = |id: &'static str, icon, label| IconButton::new(id, icon).size(ControlSize::Lg).tooltip(label);
    let icon = match phase {
        Phase::Running => IconName::Pause,
        Phase::Ready | Phase::Paused | Phase::Over => IconName::Play,
    };
    let (clocks, timer) = (hand.clocks.clone(), hand.timer);
    let run = button("show-timer-clock-run", icon, hand.run)
        .on_click(move |_, _, cx| clocks.update(cx, |clocks, cx| clocks.toggle(timer, cx)));
    let (clocks, id) = (hand.clocks, timer.id);
    let reset = button("show-timer-clock-reset", IconName::RotateCcw, hand.reset)
        .on_click(move |_, _, cx| clocks.update(cx, |clocks, cx| clocks.reset(id, cx)));
    let hidden = hand.edit.is_none();
    let swap = hand.swap.map(|(label, act)| {
        button("show-timer-clock-focus", IconName::RefreshCw, label).on_click(move |_, window, cx| act(window, cx))
    });
    let edit = hand.edit.map(|(label, act)| {
        button("show-timer-clock-edit", IconName::Pencil, label).on_click(move |_, window, cx| act(window, cx))
    });
    let palette = cx.theme().palette();
    let row = div()
        .id("show-timer-buttons")
        .when(!hidden, |row| row.occlude())
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .flex()
        .gap_1()
        .px_2()
        .py_0p5()
        .rounded_full()
        .bg(palette.surface)
        .border_1()
        .border_color(palette.border)
        .shadow_lg()
        .child(run)
        .child(reset)
        .children(edit)
        .children(swap);
    let row = match hidden {
        true => row.invisible().group_hover("show-timer", |row| row.visible()),
        false => row,
    };
    let band = div().absolute().h(BUTTONS).flex().items_center();
    let band = match above {
        true => band.top_0(),
        false => band.bottom_0(),
    };
    match side {
        Side::Left => band.left_0(),
        Side::Middle => band.left_0().right_0().justify_center(),
        Side::Right => band.right_0(),
    }
    .child(row)
    .into_any_element()
}
