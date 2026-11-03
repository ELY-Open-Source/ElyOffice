//! Dragging the cursor's table borders (DOC-07).

use gpui::{Bounds, Context, CursorStyle, Pixels, Point, px, size};

use super::LokView;
use crate::table::{Lines, Table};
use crate::{Doc, LokError};

/// Least gap the kernel keeps between lines, in twips.
const GAP: i32 = 40;

/// A border, as `.uno:TableChangeCurrentBorderPosition` names it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Side {
    ColumnLeft,
    Column(u16),
    ColumnRight,
    Row(u16),
    RowBottom,
}

impl Side {
    /// Column lines stand upright and move across.
    fn upright(self) -> bool {
        matches!(self, Side::ColumnLeft | Side::Column(_) | Side::ColumnRight)
    }

    fn kind(self) -> &'static str {
        match self {
            Side::ColumnLeft => "column-left",
            Side::Column(_) => "column-middle",
            Side::ColumnRight => "column-right",
            Side::Row(_) => "row-middle",
            Side::RowBottom => "row-right",
        }
    }

    fn index(self) -> u16 {
        match self {
            Side::Column(index) | Side::Row(index) => index,
            _ => 0,
        }
    }
}

/// A held border, with the table read it came from.
#[derive(Clone)]
pub(super) struct Held {
    side: Side,
    /// The pointer's offset from the line when pressed.
    grab: i32,
    from: i32,
    to: i32,
    table: Table,
    doc: Doc,
}

/// The cursor's table as last read, and a held border.
#[derive(Default)]
pub(super) struct Borders {
    table: Option<(Table, Doc)>,
    /// The revision of the batch the table came from.
    read: u64,
    held: Option<Held>,
    hover: Option<CursorStyle>,
}

/// Each border: side, place, span and room, in twips.
fn lines(table: &Table) -> Vec<(Side, i32, (i32, i32), i32)> {
    let (columns, rows) = (&table.columns, &table.rows);
    let down = (rows.offset + rows.left, rows.offset + rows.right);
    let across = (columns.offset + columns.left, columns.offset + columns.right);
    let mut upright = vec![(Side::ColumnLeft, across.0)];
    upright.extend(inner(columns).map(|(index, at)| (Side::Column(index), at)));
    upright.push((Side::ColumnRight, across.1));
    let mut level: Vec<_> = inner(rows).map(|(index, at)| (Side::Row(index), at)).collect();
    level.push((Side::RowBottom, down.1));
    let mut lines = spaced(&upright, None, down);
    lines.extend(spaced(&level, Some(down.0), across));
    lines
}

/// Gives each line its span and nearest neighbour's distance.
fn spaced(stops: &[(Side, i32)], before: Option<i32>, span: (i32, i32)) -> Vec<(Side, i32, (i32, i32), i32)> {
    let places: Vec<i32> = before.into_iter().chain(stops.iter().map(|&(_, at)| at)).collect();
    let skip = places.len() - stops.len();
    let room = |i: usize| {
        let prev = i.checked_sub(1).map(|j| places[i] - places[j]);
        prev.into_iter().chain(places.get(i + 1).map(|next| next - places[i])).min().unwrap_or(i32::MAX)
    };
    stops.iter().enumerate().map(|(i, &(side, at))| (side, at, span, room(i + skip))).collect()
}

fn inner(lines: &Lines) -> impl Iterator<Item = (u16, i32)> + '_ {
    lines.inner.iter().enumerate().filter(|(_, line)| !line.hidden).map(|(index, line)| (index as u16, lines.offset + line.position))
}

/// `offset` as the kernel will clamp it.
fn clamp(table: &Table, side: Side, offset: i32) -> i32 {
    let axis = if side.upright() { &table.columns } else { &table.rows };
    let (first, last) = (axis.inner.first(), axis.inner.last());
    // Past the twips range is unbounded, as LONG_MAX.
    let twips = |bound: i64| bound.clamp(i32::MIN.into(), i32::MAX.into()) as i32;
    // Impress sends its outer bounds; Writer leaves a gap.
    let (at, low, high) = match (side, axis.ends) {
        (Side::ColumnLeft, Some([(low, high), _])) => (axis.left, twips(low), twips(high)),
        (Side::ColumnRight | Side::RowBottom, Some([_, (low, high)])) => (axis.right, twips(low), twips(high)),
        (Side::ColumnLeft, None) => (axis.left, i32::MIN, first.map_or(axis.right, |line| line.position) - GAP),
        (Side::ColumnRight | Side::RowBottom, None) => {
            (axis.right, last.map_or(axis.left, |line| line.position) + GAP, i32::MAX)
        }
        (Side::Column(index) | Side::Row(index), ends) => {
            let line = axis.inner[usize::from(index)];
            let (min, max) = (twips(line.min), twips(line.max));
            let high = if ends.is_some() { max } else { max.saturating_sub(GAP).max(min) };
            (line.position, min, high)
        }
    };
    let moved = (at.saturating_add(offset)).clamp(low, high) - at;
    // A line past its bound never moves against the drag.
    if moved.signum() == offset.signum() { moved } else { 0 }
}

impl LokView {
    /// Takes the cursor's table from a batch at `revision`.
    pub(super) fn read_borders(&mut self, table: Option<Table>, revision: u64) {
        if revision < self.borders.read {
            log::debug!("lok: table read at {revision} is older than {}", self.borders.read);
            return;
        }
        if table.is_none() {
            self.borders.hover = None;
        }
        let mut doc = self.doc.clone();
        doc.rev = revision;
        self.borders.read = revision;
        self.borders.table = table.map(|table| (table, doc));
    }

    /// The border under `position`; short cells keep their middles.
    fn border_at(&self, position: Point<Pixels>) -> Option<(Side, i32)> {
        let (table, _) = self.borders.table.as_ref()?;
        lines(table).into_iter().find_map(|(side, at, (start, end), room)| {
            let reach = self.handle_size.min(px(self.geom.px_of(room) / 4.0));
            let (from, to) = if side.upright() {
                (self.hit_point((at, start)), self.hit_point((at, end)))
            } else {
                (self.hit_point((start, at)), self.hit_point((end, at)))
            };
            let (near, along, low, high) = if side.upright() {
                ((from.x - position.x).abs(), position.y, from.y, to.y)
            } else {
                ((from.y - position.y).abs(), position.x, from.x, to.x)
            };
            (near <= reach && along >= low - reach && along <= high + reach).then_some((side, at))
        })
    }

    /// Holds the border under `position`, if any.
    pub(super) fn hold_border(&mut self, position: Point<Pixels>) -> bool {
        let Some(((side, at), (table, doc))) = self.border_at(position).zip(self.borders.table.clone()) else {
            return false;
        };
        let (x, y) = self.doc_point(position);
        let grab = if side.upright() { x } else { y } - at;
        log::info!("lok: table border {} {} held", side.kind(), side.index());
        self.borders.held = Some(Held { side, grab, from: at, to: at, table, doc });
        true
    }

    /// A held border follows the pointer; a hovered one shows.
    pub(super) fn move_border(&mut self, position: Point<Pixels>) -> bool {
        let Some(held) = &self.borders.held else {
            let side = self.border_at(position).map(|(side, _)| side);
            self.borders.hover = side.map(|side| if side.upright() { CursorStyle::ResizeColumn } else { CursorStyle::ResizeRow });
            return false;
        };
        let (x, y) = self.doc_point(position);
        let want = if held.side.upright() { x } else { y } - held.grab;
        let to = held.from + clamp(&held.table, held.side, want - held.from);
        if let Some(held) = self.borders.held.as_mut() {
            held.to = to;
        }
        true
    }

    /// Lets go at `position`: one command at the read's revision.
    pub(super) fn drop_border(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) -> bool {
        if !self.move_border(position) {
            return false;
        }
        let Some(held) = self.borders.held.take() else {
            return false;
        };
        let offset = held.to - held.from;
        if offset == 0 {
            log::info!("lok: table border let go in place");
            return true;
        }
        log::info!("lok: table border {} {} moves {offset}", held.side.kind(), held.side.index());
        let (index, offset) = (held.side.index().to_string(), offset.to_string());
        if let Err(error) = self.run_at(&held.doc, "table.border", &[held.side.kind(), &index, &offset]) {
            cx.emit(LokError(format!("the table border did not move: {error}")));
        }
        true
    }

    pub(super) fn holds_border(&self) -> bool {
        self.borders.held.is_some()
    }

    /// The pointer a border asks for, while held or hovered.
    pub(super) fn border_pointer(&self) -> Option<CursorStyle> {
        match &self.borders.held {
            Some(held) if held.side.upright() => Some(CursorStyle::ResizeColumn),
            Some(_) => Some(CursorStyle::ResizeRow),
            None => self.borders.hover,
        }
    }

    /// The guide line of a held border, in window pixels.
    pub(super) fn guide(&self) -> Option<Bounds<Pixels>> {
        let held = self.borders.held.as_ref()?;
        let (_, _, (start, end), _) = lines(&held.table).into_iter().find(|&(side, ..)| side == held.side)?;
        Some(if held.side.upright() {
            let (top, bottom) = (self.window_point((held.to, start)), self.window_point((held.to, end)));
            Bounds { origin: top, size: size(px(1.0), bottom.y - top.y) }
        } else {
            let (left, right) = (self.window_point((start, held.to)), self.window_point((end, held.to)));
            Bounds { origin: left, size: size(right.x - left.x, px(1.0)) }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::Line;

    fn table() -> Table {
        let line = |position, min, max| Line { position, min, max, hidden: false };
        Table {
            columns: Lines { left: 0, right: 9000, offset: 1000, inner: vec![line(3000, 300, 6000), line(6000, 3300, 8700)], ends: None },
            rows: Lines { left: 0, right: 800, offset: 2000, inner: vec![line(400, 100, 800)], ends: None },
        }
    }

    #[test]
    fn borders_lie_where_the_kernel_puts_them() {
        let sides: Vec<_> = lines(&table()).into_iter().map(|(side, at, _, room)| (side, at, room)).collect();
        assert_eq!(
            sides,
            [
                (Side::ColumnLeft, 1000, 3000),
                (Side::Column(0), 4000, 3000),
                (Side::Column(1), 7000, 3000),
                (Side::ColumnRight, 10_000, 3000),
                (Side::Row(0), 2400, 400),
                (Side::RowBottom, 2800, 400),
            ]
        );
    }

    #[test]
    fn a_drag_clamps_as_the_kernel_does() {
        let table = table();
        assert_eq!(clamp(&table, Side::Column(0), 500), 500, "inside its bounds");
        assert_eq!(clamp(&table, Side::Column(0), 9000), 2960, "up to max less the gap");
        assert_eq!(clamp(&table, Side::Column(1), -9000), -2700, "down to min");
        assert_eq!(clamp(&table, Side::ColumnLeft, 5000), 2960, "short of the first line");
        assert_eq!(clamp(&table, Side::ColumnRight, -8000), -2960, "past the last line");
        assert_eq!(clamp(&table, Side::RowBottom, -800), -360, "a row keeps the gap");
        assert_eq!(clamp(&table, Side::Row(0), 500), 360, "max less the gap");
    }

    #[test]
    fn sent_ends_bound_the_outer_lines() {
        let mut table = table();
        table.columns.ends = Some([(-50_000, 2800), (6200, 50_000)]);
        table.rows.ends = Some([(-50_000, 380), (717, 50_000)]);
        assert_eq!(clamp(&table, Side::ColumnLeft, 5000), 2800, "to the first column's minimum");
        assert_eq!(clamp(&table, Side::ColumnLeft, -900), -900, "outward too");
        assert_eq!(clamp(&table, Side::ColumnRight, -8000), -2800, "to the last column's minimum");
        assert_eq!(clamp(&table, Side::RowBottom, -800), -83, "to the last row's minimum");
        assert_eq!(clamp(&table, Side::Column(0), 9000), 3000, "Impress's max needs no gap");
    }

    #[test]
    fn a_long_max_bound_is_no_bound() {
        let mut table = table();
        table.rows.inner[0].max = i64::MAX;
        assert_eq!(clamp(&table, Side::Row(0), 5000), 5000, "a row grows freely");
        table.rows.inner[0].min = i64::MIN;
        assert_eq!(clamp(&table, Side::Row(0), -5000), -5000, "and shrinks past zero");
    }

    #[test]
    fn a_line_at_or_past_its_bound_stays_put() {
        let mut table = table();
        table.columns.inner[0].max = 3000;
        assert_eq!(clamp(&table, Side::Column(0), 0), 0, "a still press sends nothing");
        assert_eq!(clamp(&table, Side::Column(0), 200), 0, "no room toward the bound");
        assert_eq!(clamp(&table, Side::Column(0), -200), -200, "away from it is free");
    }
}
