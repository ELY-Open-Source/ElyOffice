//! What lies over a slide: its scores and countdown.

use gpui::{AnyElement, App, Pixels};

use super::scoreboard::{self, Band, Board};
use super::timer::{self, Face, Hand, Spot};

/// Slide `part`'s overlays on a slide `w` by `h`.
pub fn overlays(board: Option<&Board>, face: Option<(Face, Hand)>, part: usize, size: (Pixels, Pixels), cx: &App) -> Vec<AnyElement> {
    let beside = board.is_some_and(|board| board.centred(part));
    let tall = size.1 * timer::BAND;
    let band = match face.as_ref().filter(|(face, _)| beside && face.at.is_none()).map(|(face, _)| face.spot) {
        None => Band::None,
        Some(Spot::Corner) => Band::Bottom(tall),
        Some(Spot::Top | Spot::Center) => Band::Top(tall),
    };
    let scores = board.and_then(|board| scoreboard::overlay(board, part, size, band, cx));
    let countdown = face.map(|(face, hand)| timer::face(&face, hand, size, beside, cx));
    scores.into_iter().chain(countdown).collect()
}
