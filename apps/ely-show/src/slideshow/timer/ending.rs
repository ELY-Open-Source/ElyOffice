//! What a done countdown shows over its slide.

use std::{f32::consts::TAU, time::Duration};

use ely_gpui_component::motion;
use gpui::{
    Animation, AnimationExt as _, AnyElement, App, Div, IntoElement as _, ParentElement as _, Pixels,
    Styled as _, div, px, rgb,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Ending {
    Nothing,
    Flash,
    #[default]
    Confetti,
    Balloons,
}

/// Each effect, its kept word, its label's key.
pub const ENDINGS: [(Ending, &str, &str); 4] = [
    (Ending::Nothing, "none", "show.timer.effect.none"),
    (Ending::Flash, "flash", "show.timer.effect.flash"),
    (Ending::Confetti, "confetti", "show.timer.effect.confetti"),
    (Ending::Balloons, "balloons", "show.timer.effect.balloons"),
];

/// How long an effect stays over its slide.
pub const LASTS: Duration = Duration::from_secs(10);
/// Flashes in that time.
const FLASHES: f32 = 8.0;
/// How strongly a flash covers the slide at most.
const COVER: f32 = 0.45;
/// The one moment an effect shows.
const STILL: f32 = 0.4;
// Fixed: party colors, whatever the theme.
const PARTY: [u32; 6] = [0xe53935, 0xfb8c00, 0xfdd835, 0x43a047, 0x1e88e5, 0x8e24aa];

/// Draws an effect at a moment, onto its cover.
type Draw = fn(Div, f32, Pixels, Pixels) -> Div;

/// The effect over a slide `w` by `h`.
pub fn over(ending: Ending, (w, h): (Pixels, Pixels), cx: &App) -> Option<AnyElement> {
    let (id, draw): (&'static str, Draw) = match ending {
        Ending::Nothing => return None,
        Ending::Flash => ("show-timer-flash", flash),
        Ending::Confetti => ("show-timer-confetti", confetti),
        Ending::Balloons => ("show-timer-balloons", balloons),
    };
    // Clicks pass through: nothing here takes them.
    let still = draw(div().absolute().top_0().left_0().w(w).h(h).overflow_hidden(), STILL, w, h);
    // Fades in, then holds still.
    let come = Animation::new(motion::duration(motion::BASE, cx));
    Some(still.with_animation(id, come, |still, t| still.opacity(t)).into_any_element())
}

fn flash(cover: Div, t: f32, _: Pixels, _: Pixels) -> Div {
    let strength = COVER * (t * FLASHES * TAU / 2.0).sin().abs();
    cover.child(div().size_full().bg(rgb(0xd32626)).opacity(strength))
}

/// Piece `n`'s share of one kind, 0 to 1.
fn share(n: usize, kind: u32) -> f32 {
    // A bit mix, so no two traits line up.
    let mut mixed = (n as u32).wrapping_mul(0x9e37_79b9) ^ kind.wrapping_mul(0x85eb_ca6b);
    mixed = (mixed ^ (mixed >> 16)).wrapping_mul(0x7feb_352d);
    mixed = (mixed ^ (mixed >> 15)).wrapping_mul(0x846c_a68b);
    (mixed ^ (mixed >> 16)) as f32 / u32::MAX as f32
}

// What each share decides.
const START: u32 = 1;
const ACROSS: u32 = 2;
const SIZE: u32 = 3;

/// How far piece `n` is through one pass.
fn pass(n: usize, t: f32, speed: f32, late: f32, passes: f32) -> Option<f32> {
    let gone = t * speed - share(n, START) * late;
    (0.0..passes).contains(&gone).then(|| gone.fract())
}

fn confetti(cover: Div, t: f32, w: Pixels, h: Pixels) -> Div {
    let size = h * 0.022;
    cover.children((0..90).filter_map(|n| {
        let down = pass(n, t, 4.0, 0.8, 3.0)?;
        let sway = (down * TAU * 2.0 + n as f32).sin() * 0.015;
        // Strips seem to tumble as they widen and thin.
        let wide = size * (0.35 + 0.65 * (down * TAU * 3.0 + n as f32).cos().abs());
        let piece = div().absolute().left(w * (share(n, ACROSS) + sway)).top((h + size * 2.0) * down - size);
        Some(piece.w(wide).h(size).rounded(px(2.0)).bg(rgb(PARTY[n % PARTY.len()])))
    }))
}

fn balloons(cover: Div, t: f32, w: Pixels, h: Pixels) -> Div {
    cover.children((0..16).filter_map(|n| {
        let up = pass(n, t, 2.4, 0.4, 2.0)?;
        let wide = h * (0.09 + 0.05 * share(n, SIZE));
        let string = wide * 0.9;
        let tall = wide + string;
        let sway = (up * TAU * 1.5 + n as f32).sin() * 0.012;
        let color = rgb(PARTY[n % PARTY.len()]);
        let balloon = div().absolute().left(w * (share(n, ACROSS) * 0.94 + sway)).top(h - (h + tall) * up);
        let shine = div().absolute().top(wide * 0.18).left(wide * 0.2).size(wide * 0.2).rounded_full();
        let round = div().relative().size(wide).rounded_full().bg(color).child(shine.bg(rgb(0xffffff)).opacity(0.55));
        Some(balloon.flex().flex_col().items_center().child(round).child(div().w(px(2.0)).h(string).bg(color)))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_still_moment_shows_each_effect() {
        let shown = |pieces, speed, late, passes| (0..pieces).filter(|&n| pass(n, STILL, speed, late, passes).is_some()).count();
        assert!(shown(90, 4.0, 0.8, 3.0) > 45, "most confetti is in the air");
        assert!(shown(16, 2.4, 0.4, 2.0) > 8, "most balloons are rising");
        assert!((STILL * FLASHES * TAU / 2.0).sin().abs() > 0.5, "the flash is lit");
    }
}
