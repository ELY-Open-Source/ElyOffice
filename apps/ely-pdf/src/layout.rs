//! Where pages sit for a mode, zoom and turn (PDF-01).

/// How pages follow one another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Continuous,
    Single,
    Two,
    /// One row, scrolled across.
    Horizontal,
}

/// How big pages are drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Zoom {
    Width,
    Page,
    /// Pixels a point; 96 / 72 is actual size.
    Scale(f32),
}

/// Points to pixels at actual size.
pub const ACTUAL: f32 = 96.0 / 72.0;
/// Space around and between pages, in pixels.
pub const GAP: f32 = 16.0;

/// One page placed, in pixels from the top left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    pub page: usize,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// The pages placed, the whole size, the scale used.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub placed: Vec<Placed>,
    pub size: (f32, f32),
    pub scale: f32,
}

/// A page's size in points once turned.
pub fn turned((w, h): (f32, f32), rotate: i32) -> (f32, f32) {
    match rotate.rem_euclid(360) {
        90 | 270 => (h, w),
        _ => (w, h),
    }
}

/// Lays `sizes` out in `view` pixels; `current` shows alone.
pub fn layout(sizes: &[(f32, f32)], rotate: i32, (mode, zoom): (Mode, Zoom), view: (f32, f32), current: usize) -> Layout {
    let sizes: Vec<(f32, f32)> = sizes.iter().map(|size| turned(*size, rotate)).collect();
    let shown: Vec<usize> = match mode {
        Mode::Single => sizes.len().checked_sub(1).map(|last| current.min(last)).into_iter().collect(),
        _ => (0..sizes.len()).collect(),
    };
    let across = if mode == Mode::Two { 2.0 } else { 1.0 };
    // Rows of one or two pages, as wide as theirs.
    let rows: Vec<Vec<usize>> = match mode {
        Mode::Two => shown.chunks(2).map(<[usize]>::to_vec).collect(),
        Mode::Horizontal => vec![shown.clone()],
        _ => shown.iter().map(|page| vec![*page]).collect(),
    };
    // A row of pages fits one page, not the row.
    let fitted = match mode {
        Mode::Horizontal => shown.iter().map(|page| sizes[*page].0).collect(),
        _ => rows.iter().map(|row| row.iter().map(|page| sizes[*page].0).sum::<f32>()).collect::<Vec<_>>(),
    };
    let widest = fitted.into_iter().fold(0.0, f32::max).max(1.0);
    let room = (view.0 - GAP * (1.0 + across)).max(1.0);
    let scale = match zoom {
        Zoom::Scale(scale) => scale,
        Zoom::Width => room / widest,
        Zoom::Page => {
            let tallest = shown.iter().map(|page| sizes[*page].1).fold(0.0, f32::max).max(1.0);
            (room / widest).min((view.1 - 2.0 * GAP).max(1.0) / tallest)
        }
    };
    let width = match mode {
        Mode::Horizontal => shown.iter().map(|page| sizes[*page].0 * scale + GAP).sum::<f32>() + GAP,
        _ => widest * scale + GAP * (1.0 + across),
    }
    .max(view.0);
    let (mut placed, mut y) = (Vec::new(), GAP);
    for row in rows {
        let row_w = row.iter().map(|page| sizes[*page].0 * scale).sum::<f32>() + GAP * (row.len() as f32 - 1.0);
        let row_h = row.iter().map(|page| sizes[*page].1 * scale).fold(0.0, f32::max);
        let mut x = (width - row_w) / 2.0;
        for page in row {
            let (w, h) = (sizes[page].0 * scale, sizes[page].1 * scale);
            placed.push(Placed { page, x, y: y + (row_h - h) / 2.0, w, h });
            x += w + GAP;
        }
        y += row_h + GAP;
    }
    Layout { placed, size: (width, y.max(view.1)), scale }
}

/// Places on screen between `top` and `bottom`.
pub fn visible(layout: &Layout, top: f32, bottom: f32) -> impl Iterator<Item = &Placed> {
    layout.placed.iter().filter(move |place| place.y < bottom && place.y + place.h > top)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LETTER: (f32, f32) = (612.0, 792.0);

    #[test]
    fn continuous_fits_the_width_and_stacks_with_gaps() {
        let out = layout(&[LETTER, LETTER], 0, (Mode::Continuous, Zoom::Width), (644.0, 500.0), 0);
        assert_eq!(out.scale, 1.0, "612 points in 612 pixels of room");
        assert_eq!(out.placed[0], Placed { page: 0, x: 16.0, y: 16.0, w: 612.0, h: 792.0 });
        assert_eq!(out.placed[1].y, 16.0 + 792.0 + 16.0);
        assert_eq!(out.size, (644.0, 16.0 + 2.0 * (792.0 + 16.0)));
    }

    #[test]
    fn single_shows_one_page_and_two_pairs_them() {
        let pages = [LETTER, LETTER, LETTER];
        let one = layout(&pages, 0, (Mode::Single, Zoom::Scale(1.0)), (800.0, 600.0), 2);
        assert_eq!(one.placed.iter().map(|p| p.page).collect::<Vec<_>>(), [2]);
        let two = layout(&pages, 0, (Mode::Two, Zoom::Scale(1.0)), (800.0, 600.0), 0);
        let rows: Vec<(usize, f32)> = two.placed.iter().map(|p| (p.page, p.y)).collect();
        assert_eq!(rows, [(0, 16.0), (1, 16.0), (2, 824.0)]);
        assert_eq!(two.placed[1].x - two.placed[0].x, 612.0 + 16.0, "side by side");
    }

    #[test]
    fn horizontal_puts_pages_in_one_row_and_fits_one_page() {
        let out = layout(&[LETTER, LETTER, LETTER], 0, (Mode::Horizontal, Zoom::Width), (644.0, 500.0), 0);
        assert_eq!(out.scale, 1.0, "one page fills the room");
        let at: Vec<(f32, f32)> = out.placed.iter().map(|p| (p.x, p.y)).collect();
        assert_eq!(at, [(16.0, 16.0), (644.0, 16.0), (1272.0, 16.0)]);
        assert_eq!(out.size, (16.0 + 3.0 * (612.0 + 16.0), 16.0 + 792.0 + 16.0));
        let page = layout(&[LETTER, (612.0, 1584.0)], 0, (Mode::Horizontal, Zoom::Page), (644.0, 824.0), 0);
        assert_eq!(page.scale, 0.5, "the tallest page fits the height");
    }

    #[test]
    fn a_quarter_turn_swaps_sides_and_page_zoom_fits_the_height() {
        let out = layout(&[LETTER], 90, (Mode::Continuous, Zoom::Page), (1000.0, 644.0), 0);
        assert_eq!((out.placed[0].w / out.scale, out.placed[0].h / out.scale), (792.0, 612.0));
        assert!((out.placed[0].h - 612.0).abs() < 0.01, "the height fits");
        let shown: Vec<usize> = visible(&out, 0.0, 10.0).map(|p| p.page).collect();
        assert!(shown.is_empty(), "above the first page");
    }

    #[test]
    fn no_pages_lay_out_as_nothing() {
        for mode in [Mode::Continuous, Mode::Single, Mode::Two] {
            assert!(layout(&[], 0, (mode, Zoom::Page), (800.0, 600.0), 0).placed.is_empty());
        }
    }
}
