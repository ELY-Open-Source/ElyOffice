//! Where a carried frame snaps, and its lines.

use crate::Rect;

/// What a carried frame may snap to, in twips.
pub(super) struct Targets<'a> {
    pub(super) slide: (i32, i32),
    pub(super) others: &'a [Rect],
    /// Vertical guides' x, then horizontal guides' y.
    pub(super) guides: (Vec<i32>, Vec<i32>),
    pub(super) grid: Option<(i32, i32)>,
    /// How near counts as aligned.
    pub(super) reach: i32,
}

/// A rect on one axis: start, length, cross start, length.
type Span = (i32, i32, i32, i32);

/// A line on one axis: along `a`, across `c`.
type Seg = ((i32, i32), (i32, i32));

/// The snapped frame and its dashed lines.
pub(super) fn snap(frame: Rect, targets: &Targets) -> (Rect, Vec<Rect>) {
    let (x, y, w, h) = frame;
    let across = |(x, y, w, h): Rect| (x, w, y, h);
    let down = |(x, y, w, h): Rect| (y, h, x, w);
    let xs: Vec<Span> = targets.others.iter().map(|&rect| across(rect)).collect();
    let ys: Vec<Span> = targets.others.iter().map(|&rect| down(rect)).collect();
    let grid = targets.grid.unzip();
    let (dx, x_segs) = axis(across(frame), &xs, (targets.slide.0, targets.slide.1), &targets.guides.0, grid.0, targets.reach);
    let (dy, y_segs) = axis(down(frame), &ys, (targets.slide.1, targets.slide.0), &targets.guides.1, grid.1, targets.reach);
    let mut lines: Vec<Rect> = x_segs.into_iter().map(|((a0, a1), (c0, c1))| (a0, c0, a1 - a0, c1 - c0)).collect();
    lines.extend(y_segs.into_iter().map(|((a0, a1), (c0, c1))| (c0, a0, c1 - c0, a1 - a0)));
    ((x + dx, y + dy, w, h), lines)
}

/// The shift on one axis and the lines it earns.
fn axis(
    (start, len, cross, breadth): Span,
    others: &[Span],
    (slide, slide_cross): (i32, i32),
    guides: &[i32],
    grid: Option<i32>,
    reach: i32,
) -> (i32, Vec<Seg>) {
    let points = |s: i32, l: i32| [s, s + l / 2, s + l];
    let mut lines: Vec<i32> = points(0, slide).to_vec();
    lines.extend(guides);
    lines.extend(others.iter().flat_map(|&(s, l, _, _)| points(s, l)));
    let aligned = points(start, len)
        .into_iter()
        .flat_map(|at| lines.iter().map(move |&line| line - at));
    let row: Vec<Span> = others.iter().copied().filter(|o| o.2 < cross + breadth && cross < o.2 + o.3).collect();
    let spaced = gaps(start, len, &row);
    let best = aligned.chain(spaced).filter(|d| d.abs() <= reach).min_by_key(|d| d.abs());
    let Some(shift) = best else {
        let snapped = grid.filter(|&step| step > 0).map(|step| (start as f64 / step as f64).round() as i32 * step);
        return (snapped.map_or(0, |at| at - start), Vec::new());
    };
    let (start, mid) = (start + shift, cross + breadth / 2);
    let mut segs = Vec::new();
    for at in points(start, len) {
        if points(0, slide).contains(&at) {
            segs.push(((at, at), (0, slide_cross)));
        }
        for &(_, _, c, b) in others.iter().filter(|&&(s, l, _, _)| points(s, l).contains(&at)) {
            segs.push(((at, at), (c.min(cross), (c + b).max(cross + breadth))));
        }
    }
    if let Some((before, after)) = neighbours(start, len, &row)
        && start - before == after - (start + len)
    {
        segs.push(((before, start), (mid, mid)));
        segs.push(((start + len, after), (mid, mid)));
    }
    (shift, segs)
}

/// The nearest ends before and after the span.
fn neighbours(start: i32, len: i32, row: &[Span]) -> Option<(i32, i32)> {
    let before = row.iter().map(|o| o.0 + o.1).filter(|&end| end <= start).max()?;
    let after = row.iter().map(|o| o.0).filter(|&s| s >= start + len).min()?;
    Some((before, after))
}

/// The shift that leaves equal gaps to both neighbours.
fn gaps(start: i32, len: i32, row: &[Span]) -> Option<i32> {
    let (before, after) = neighbours(start, len, row)?;
    Some((before + after - len) / 2 - start)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn targets(others: &[Rect]) -> Targets<'_> {
        Targets { slide: (10_000, 6_000), others, guides: (Vec::new(), Vec::new()), grid: None, reach: 50 }
    }

    #[test]
    fn edges_and_centres_align_within_reach() {
        let others = [(1_000, 1_000, 2_000, 1_000)];
        let (moved, lines) = snap((1_030, 3_000, 500, 500), &targets(&others));
        assert_eq!(moved.0, 1_000, "left edges meet");
        assert!(lines.iter().any(|&(x, _, w, _)| x == 1_000 && w == 0), "a line at x 1000: {lines:?}");
        let (moved, _) = snap((1_780, 3_000, 500, 500), &targets(&others));
        assert_eq!(moved.0 + 250, 2_000, "centres meet");
        let (moved, lines) = snap((1_200, 3_200, 500, 500), &targets(&others));
        assert_eq!((moved, lines.len()), ((1_200, 3_200, 500, 500), 0), "out of reach, untouched");
    }

    #[test]
    fn the_slide_centre_and_edges_align() {
        let (moved, lines) = snap((4_760, 2_760, 500, 500), &targets(&[]));
        assert_eq!((moved.0, moved.1), (4_750, 2_750), "the centre of a 10000 by 6000 slide");
        assert!(lines.contains(&(5_000, 0, 0, 6_000)) && lines.contains(&(0, 3_000, 10_000, 0)), "{lines:?}");
        assert_eq!(snap((-30, 5_530, 500, 500), &targets(&[])).0, (0, 5_500, 500, 500), "edges");
    }

    #[test]
    fn equal_gaps_to_both_neighbours_are_shown() {
        let others = [(0, 1_000, 1_000, 500), (3_000, 1_000, 1_000, 500)];
        let (moved, lines) = snap((1_730, 1_000, 500, 500), &targets(&others));
        assert_eq!(moved.0, 1_750, "750 each side");
        assert!(lines.contains(&(1_000, 1_250, 750, 0)) && lines.contains(&(2_250, 1_250, 750, 0)), "{lines:?}");
    }

    #[test]
    fn the_grid_takes_what_nothing_else_does() {
        let mut far = targets(&[]);
        far.grid = Some((1_000, 1_000));
        assert_eq!(snap((1_400, 1_600, 300, 300), &far).0, (1_000, 2_000, 300, 300));
        far.guides = (vec![1_390], Vec::new());
        assert_eq!(snap((1_400, 1_600, 300, 300), &far).0.0, 1_390, "a guide outranks the grid");
    }
}
