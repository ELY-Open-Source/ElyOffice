//! Picked lines redrawn: white text on the selection colour.

use std::collections::HashMap;
use std::sync::Arc;

use ely_gpui_component::primitives::Retired;
use ely_gpui_component::theme::ActiveTheme as _;
use gpui::{AnyElement, App, Hsla, ImageId, IntoElement as _, RenderImage, Rgba, Styled as _, canvas, px};

use crate::layout::Placed;
use crate::pages::{picture, rect_on};
use crate::view::PdfView;

/// The lightest a fill may be and still show white.
const DARKEST_ENOUGH: f32 = 110.0;
/// Below this contrast is paper grain, not ink.
const FAINT: u8 = 64;

/// A tile cut: source, pixel box, fill.
type Cut = (ImageId, [u32; 4], [u8; 3]);

/// The recoloured cuts on screen.
#[derive(Default)]
pub struct Lit {
    drawn: HashMap<Cut, Arc<RenderImage>>,
    before: HashMap<Cut, Arc<RenderImage>>,
}

impl Lit {
    /// Starts a frame; the last one's cuts may be reused.
    pub fn begin(&mut self) {
        self.before = std::mem::take(&mut self.drawn);
    }

    /// Ends a frame; unused cuts are freed.
    pub fn end(&mut self, retired: &mut Retired) {
        self.before.drain().for_each(|(_, image)| retired.push(image));
    }

    /// Every cut, for a closing view to free.
    pub fn retire(&mut self, retired: &mut Retired) {
        self.end(retired);
        self.drawn.drain().for_each(|(_, image)| retired.push(image));
    }

    fn cut(&mut self, source: &RenderImage, area: [u32; 4], fill: [u8; 3]) -> Option<Arc<RenderImage>> {
        let key = (source.id, area, fill);
        if let Some(image) = self.drawn.get(&key) {
            return Some(image.clone());
        }
        let image = match self.before.remove(&key) {
            Some(image) => image,
            None => {
                let width = source.size(0).width.0 as usize;
                let [x0, y0, x1, y1] = area.map(|v| v as usize);
                let bytes = source.as_bytes(0)?;
                let cut: Vec<u8> = (y0..y1).flat_map(|y| &bytes[(y * width + x0) * 4..(y * width + x1) * 4]).copied().collect();
                picture((area[2] - area[0], area[3] - area[1]), recolour(cut, fill))?
            }
        };
        self.drawn.insert(key, image.clone());
        Some(image)
    }
}

/// `color` as opaque BGR, darkened until white reads on it.
fn fill_of(color: Hsla) -> [u8; 3] {
    let rgb = Rgba::from(color);
    let bgr = [rgb.b, rgb.g, rgb.r].map(|v| v * 255.0);
    let keep = (DARKEST_ENOUGH / f32::from(luma(bgr.map(|v| v as u8))).max(1.0)).min(1.0);
    bgr.map(|v| (v * keep).round() as u8)
}

fn luma([b, g, r]: [u8; 3]) -> u8 {
    ((u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114) / 1000) as u8
}

/// Turns BGRA paper into `fill` and its ink into white.
fn recolour(mut bgra: Vec<u8>, fill: [u8; 3]) -> Vec<u8> {
    let tone = |p: &[u8]| luma([p[0], p[1], p[2]]);
    let mut counts = [0u32; 256];
    bgra.as_chunks::<4>().0.iter().for_each(|p| counts[usize::from(tone(p))] += 1);
    // Most pixels are paper, light or dark.
    let paper = (0..=255u8).max_by_key(|&v| counts[usize::from(v)]).unwrap_or(u8::MAX);
    let reach = bgra.as_chunks::<4>().0.iter().map(|p| tone(p).abs_diff(paper)).max().unwrap_or(0).max(FAINT);
    for p in bgra.as_chunks_mut::<4>().0 {
        let ink = f32::from(tone(p).abs_diff(paper)) / f32::from(reach);
        for (channel, base) in p.iter_mut().zip(fill) {
            *channel = (f32::from(base) + (255.0 - f32::from(base)) * ink).round() as u8;
        }
        p[3] = u8::MAX;
    }
    bgra
}

/// An image stretched over a box, in page pixels.
fn laid(image: Arc<RenderImage>, [x, y, w, h]: [f32; 4]) -> AnyElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            if let Err(error) = window.paint_image(bounds, bounds, Default::default(), image.clone(), 0, false) {
                log::error!("pdf: drawing picked text failed: {error}");
            }
        },
    )
    .absolute()
    .left(px(x))
    .top(px(y))
    .w(px(w))
    .h(px(h))
    .into_any_element()
}

impl PdfView {
    /// A page's picked lines, cut from its tiles.
    pub fn picked_text(&mut self, place: &Placed, pieces: &[(Arc<RenderImage>, [f32; 4])], scale: f32, cx: &App) -> Vec<AnyElement> {
        let lines = self.picked_lines(place.page);
        let fill = fill_of(cx.theme().colors.selection);
        let size = self.sizes[place.page];
        let mut drawn = Vec::new();
        for line in lines {
            let (x, y, w, h) = rect_on(line, size, self.rotate, scale);
            for (source, at) in pieces {
                let (left, top, wide, tall) = (at[0] * place.w, at[1] * place.h, at[2] * place.w, at[3] * place.h);
                let pixels = source.size(0);
                let (pw, ph) = (pixels.width.0 as f32, pixels.height.0 as f32);
                let (sx, sy) = (pw / wide, ph / tall);
                let area = [
                    ((x - left) * sx).floor().clamp(0.0, pw) as u32,
                    ((y - top) * sy).floor().clamp(0.0, ph) as u32,
                    ((x + w - left) * sx).ceil().clamp(0.0, pw) as u32,
                    ((y + h - top) * sy).ceil().clamp(0.0, ph) as u32,
                ];
                if area[2] <= area[0] || area[3] <= area[1] {
                    continue;
                }
                let Some(image) = self.lit.cut(source, area, fill) else {
                    log::error!("pdf: a tile of page {} would not cut", place.page);
                    continue;
                };
                let [x0, y0, x1, y1] = area.map(|v| v as f32);
                drawn.push(laid(image, [left + x0 / sx, top + y0 / sy, (x1 - x0) / sx, (y1 - y0) / sy]));
            }
        }
        drawn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ink_turns_white_and_paper_takes_the_fill() {
        let fill = [60, 50, 40];
        let (paper, ink, navy) = ([255, 255, 255, 255], [0, 0, 0, 255], [95, 56, 31, 255]);
        let white = [255, 255, 255, 255];
        let filled = [60, 50, 40, 255];
        let on_light = recolour([paper, ink, paper].concat(), fill);
        assert_eq!(on_light, [filled, white, filled].concat(), "black on white");
        assert_eq!(recolour([paper, navy, paper].concat(), fill), on_light, "coloured text too");
        assert_eq!(recolour([navy, paper, navy].concat(), fill), on_light, "white on a dark cell");
        assert_eq!(recolour([paper, paper].concat(), fill), [filled, filled].concat(), "a gap stays plain");
    }

    #[test]
    fn a_pale_selection_colour_is_darkened_for_white() {
        assert!(luma(fill_of(gpui::white())) <= DARKEST_ENOUGH as u8);
        assert_eq!(fill_of(gpui::rgb(0x403e41).into()), [0x41, 0x3e, 0x40], "a dark one is kept");
    }
}
