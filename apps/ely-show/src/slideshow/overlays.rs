//! What lies over a slide: its scores and countdown.

use gpui::{AnyElement, App, Pixels};

use super::scoreboard::{self, Band, Board};
use super::timer::{self, Face, Hand, Room, Spot};

/// Slide `part`'s overlays on a slide `w` by `h`.
pub fn overlays(board: Option<&Board>, face: Option<(Face, Hand)>, part: usize, size: (Pixels, Pixels), cx: &App) -> Vec<AnyElement> {
    let linked = board.is_some_and(|board| board.linked(part));
    let room = match face.as_ref() {
        Some((face, _)) if linked && face.ahead => Room::Ahead,
        _ if board.is_some_and(|board| board.centred(part)) => Room::Beside,
        _ => Room::Alone,
    };
    let tall = size.1 * timer::BAND;
    let band = match (room, face.as_ref()) {
        (Room::Ahead, _) => Band::Aside(size.1 * timer::ASIDE),
        (Room::Beside, Some((face, _))) if face.at.is_none() => match face.spot {
            Spot::Bottom | Spot::Corner => Band::Bottom(tall),
            Spot::Top | Spot::Center => Band::Top(tall),
        },
        _ => Band::None,
    };
    let scores = board.and_then(|board| scoreboard::overlay(board, part, size, band, cx));
    let countdown = face.map(|(face, hand)| timer::face(&face, hand, size, room, cx));
    scores.into_iter().chain(countdown).collect()
}
