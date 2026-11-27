//! How a scoreboard looks over a slide.

use ely_gpui_component::theme::ActiveTheme as _;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, Div, FontWeight, IntoElement as _, ParentElement as _, Pixels, Styled as _, div,
    relative,
};

use super::{Board, Look};

/// The corner card's share of the slide.
const CORNER: f32 = 0.4;

/// Slide `part`'s scoreboard, over a slide `w` by `h`.
pub fn overlay(board: &Board, part: usize, (w, h): (Pixels, Pixels), cx: &App) -> Option<AnyElement> {
    if board.look == Look::Off || !board.linked(part) {
        return None;
    }
    let rows = board.of(part).map(|scores| &scores.rows).filter(|rows| !rows.is_empty());
    // Alone, it hides the slide even before any rows.
    if rows.is_none() && board.look != Look::Full {
        return None;
    }
    let place = div().absolute().flex();
    let centred = |place: Div| place.top_0().left_0().size_full().items_center().justify_center();
    let sized = |size| rows.map(|rows| card(rows, size, cx));
    Some(match board.look {
        Look::Corner => place.top(h * 0.03).right(w * 0.02).children(sized((w * CORNER, h * CORNER))),
        Look::Full => centred(place).bg(cx.theme().palette().surface).children(sized((w, h))),
        Look::Center | Look::Off => centred(place).children(sized((w, h))),
    }
    .into_any_element())
}

fn figure(cell: &str) -> bool {
    cell.chars().any(|c| c.is_ascii_digit())
}

/// A first row without figures titles the columns.
fn titled(rows: &[Vec<String>]) -> bool {
    rows.len() > 1 && rows[0].len() > 1 && !rows[0][1..].iter().any(|cell| figure(cell))
}

/// A few teams with one score stand side by side.
fn side_by_side(rows: &[Vec<String>]) -> bool {
    (2..=4).contains(&rows.len()) && rows[0].len() == 2 && !titled(rows)
}

/// The card sized to fit a `w` by `h` box.
fn card(rows: &[Vec<String>], size: (Pixels, Pixels), cx: &App) -> Div {
    let palette = cx.theme().palette();
    let body = match side_by_side(rows) {
        true => tiles(rows, size, cx),
        false => table(rows, size, cx),
    };
    body.overflow_hidden()
        .rounded(size.1 * 0.035)
        .bg(palette.surface)
        .border_1()
        .border_color(palette.border)
        .text_color(palette.fg)
        .shadow_lg()
}

/// A short line, clear of the rounded corners.
fn mark(wide: Pixels, h: Pixels, cx: &App) -> Div {
    div().flex_none().w(wide).h(h * 0.014).rounded_full().bg(cx.theme().colors.accent)
}

/// One block per team: its name over its score.
fn tiles(rows: &[Vec<String>], (w, h): (Pixels, Pixels), cx: &App) -> Div {
    let palette = cx.theme().palette();
    let wide = w * (0.86 / rows.len() as f32).min(0.36);
    // Long names and figures shrink to fit their block.
    let longest = |at: usize| rows.iter().map(|row| row[at].chars().count()).max().unwrap_or_default().max(1) as f32;
    let name = (h * 0.085).min(wide * 1.5 / longest(0));
    let score = (h * 0.27).min(wide * 1.4 / longest(1));
    div().flex().children(rows.iter().enumerate().map(|(at, row)| {
        let line = |text: Pixels, cell: &str| {
            div().w_full().h(text * 1.25).line_height(relative(1.25)).text_size(text).flex().justify_center()
                .child(div().min_w_0().truncate().child(cell.to_owned()))
        };
        div()
            .w(wide)
            .flex()
            .flex_col()
            .items_center()
            .when(at > 0, |tile| tile.border_l_1().border_color(palette.border))
            .px(wide * 0.05)
            .py(h * 0.05)
            .gap(h * 0.02)
            .child(line(name, &row[0]).font_weight(FontWeight::MEDIUM))
            .child(mark(wide * 0.3, h, cx))
            .child(line(score, &row[1]).font_weight(FontWeight::BOLD))
    }))
}

/// Banded rows: names on the left, figures aligned.
fn table(rows: &[Vec<String>], (w, h): (Pixels, Pixels), cx: &App) -> Div {
    let palette = cx.theme().palette();
    let accent = cx.theme().colors.accent;
    let lines = rows.len() as f32;
    // Text shrinks so every row stays on the slide.
    let text = (w * 0.042).min(h * 0.84 / (lines * 1.8));
    let titled = titled(rows);
    let columns = (0..rows[0].len()).map(|at| {
        let column = div().flex().flex_col().min_w_0();
        // The names take what room is left.
        let column = if at == 0 { column.flex_1() } else { column.flex_none() };
        column.children(rows.iter().enumerate().map(|(line, row)| {
            div()
                .h(text * 1.8)
                .line_height(relative(1.8))
                .px(text * 0.8)
                .flex()
                .when(at > 0, |cell| cell.justify_center())
                .when(line % 2 == 1, |cell| cell.bg(palette.border.opacity(0.35)))
                .when(titled && line == 0, |cell| cell.text_color(accent))
                .when(at > 0 || (titled && line == 0), |cell| cell.font_weight(FontWeight::BOLD))
                .child(div().min_w_0().truncate().child(row[at].clone()))
        }))
    });
    div()
        .flex()
        .flex_col()
        .min_w(w * 0.6)
        .max_w(w * 0.92)
        .items_center()
        .py(h * 0.025)
        .gap(h * 0.02)
        .text_size(text)
        .child(mark(w * 0.12, h, cx))
        .child(div().w_full().flex().children(columns))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(cells: &[&[&str]]) -> Vec<Vec<String>> {
        cells.iter().map(|row| row.iter().map(|cell| cell.to_string()).collect()).collect()
    }

    #[test]
    fn a_few_scored_teams_stand_side_by_side() {
        assert!(side_by_side(&rows(&[&["Lions", "5"], &["Eagles", "2"]])));
        assert!(side_by_side(&rows(&[&["Lions", "5"], &["Eagles", ""]])));
        assert!(!side_by_side(&rows(&[&["Team", "Points"], &["Lions", "5"]])));
        assert!(!side_by_side(&rows(&[&["Lions", "5"]])));
        assert!(!side_by_side(&rows(&[&["Lions", "5", "1"], &["Eagles", "2", "0"]])));
        assert!(!side_by_side(&rows(&[&["A", "1"], &["B", "2"], &["C", "3"], &["D", "4"], &["E", "5"]])));
    }
}
