//! An object carried by its body: shown, snapped, dropped (SHOW-03).

use gpui::{Modifiers, Pixels, Point, px};

use super::{LokView, snap};
use crate::{Doc, Rect, show::Objects};

/// Pixels a press travels before it carries.
const START: f32 = 4.0;
/// Pixels within which edges align.
const REACH: f32 = 6.0;

/// The shown slide as the app read it.
pub struct Slide {
    pub part: usize,
    /// Width and height in 1/100 mm.
    pub size: (i64, i64),
    pub objects: Objects,
    /// The document the objects were read from.
    pub at: Doc,
}

/// A pressed object, and where it is carried.
pub(super) struct Carry {
    press: Point<Pixels>,
    from: (i32, i32),
    frame: Rect,
    /// The carried objects; the drop selects them again.
    carried: Vec<usize>,
    /// The read the press saw; edits since refuse it.
    at: Doc,
    others: Vec<Rect>,
    /// The frame and its lines, once it moved.
    moved: Option<(Rect, Vec<Rect>)>,
}

fn twips(mm100: i64) -> i32 {
    (mm100 * 72 / 127) as i32
}

fn rect((x, y, w, h): (i64, i64, i64, i64)) -> Rect {
    (twips(x), twips(y), twips(w), twips(h))
}

fn holds((x, y, w, h): Rect, (px, py): (i32, i32)) -> bool {
    (x..=x + w).contains(&px) && (y..=y + h).contains(&py)
}

impl LokView {
    /// The slide the next carry reads; None stops carries.
    pub fn set_slide(&mut self, slide: Option<Slide>) {
        self.slide = slide;
    }

    /// Takes a plain press on an object for a carry.
    pub(super) fn pick(&mut self, at: Point<Pixels>, count: usize, keys: &Modifiers) -> bool {
        let plain = !(keys.control || keys.shift || keys.platform || keys.function);
        // Read since the last edit landed, or stale.
        let fresh = |slide: &&Slide| slide.part == self.part && slide.at.rev == self.doc.rev && self.settled();
        let Some(slide) = self.slide.as_ref().filter(fresh) else {
            return false;
        };
        if !self.doc.is_deck() || count != 1 || !plain {
            return false;
        }
        let from = self.doc_point(at);
        let objects = &slide.objects.objects;
        let carried = match &slide.objects.selection {
            // Text being edited keeps its caret and drags.
            Some(sel) if sel.editing => return false,
            Some(sel) if holds(rect(sel.rect), from) => {
                if sel.lock_position != Some(false) {
                    return false;
                }
                (rect(sel.rect), objects.iter().enumerate().filter(|(_, o)| o.selected).map(|(i, _)| i).collect())
            }
            _ => {
                // The topmost object; tables take their cells.
                let hit = objects.iter().rposition(|o| holds(rect(o.rect), from));
                match hit.filter(|&i| !objects[i].locked && objects[i].kind != "table") {
                    Some(i) => (rect(objects[i].rect), vec![i]),
                    None => return false,
                }
            }
        };
        let others = objects.iter().enumerate().filter(|(i, _)| !carried.1.contains(i)).map(|(_, o)| rect(o.rect)).collect();
        log::info!("lok: carry starts, objects {:?}", carried.1);
        let read = slide.at.clone();
        self.carry = Some(Carry { press: at, from, frame: carried.0, carried: carried.1, at: read, others, moved: None });
        self.dragging = true;
        true
    }

    /// Carries to `at`; false when nothing is carried.
    pub(super) fn carry_to(&mut self, at: Point<Pixels>, keys: Modifiers) -> bool {
        let Some(carry) = self.carry.as_ref() else {
            return false;
        };
        let travel = at - carry.press;
        if carry.moved.is_none() && travel.x.abs().max(travel.y.abs()) < px(START) {
            return true;
        }
        let (to, near) = (self.doc_point(at), self.doc_point(carry.press + Point::new(px(REACH), px(0.))));
        let (x, y, w, h) = carry.frame;
        let frame = (x + to.0 - carry.from.0, y + to.1 - carry.from.1, w, h);
        let moved = match (keys.alt, self.slide.as_ref()) {
            (false, Some(slide)) => snap::snap(frame, &targets(slide, &carry.others, near.0 - carry.from.0)),
            _ => (frame, Vec::new()),
        };
        if let Some(carry) = self.carry.as_mut() {
            carry.moved = Some(moved);
        }
        true
    }

    /// Drops a carry; an unmoved press clicks.
    pub(super) fn drop_carry(&mut self, release: Point<Pixels>, count: usize, keys: &Modifiers) -> bool {
        let Some(carry) = self.carry.take() else {
            return false;
        };
        self.end_drag();
        let Some(((x, y, _, _), _)) = carry.moved else {
            self.mouse(0, carry.press, 1, count, keys);
            self.mouse(1, release, 1, count, keys);
            return true;
        };
        log::info!("lok: carry drops objects {:?} at {x}, {y}", carry.carried);
        // The press's objects, whatever is selected now.
        let mut steps: Vec<(&str, Vec<String>)> = carry
            .carried
            .iter()
            .enumerate()
            .map(|(n, i)| ("object.select", vec![i.to_string(), (n > 0).to_string()]))
            .collect();
        steps.push(("object.move", vec![x.to_string(), y.to_string()]));
        let values: Vec<Vec<&str>> = steps.iter().map(|(_, v)| v.iter().map(String::as_str).collect()).collect();
        let commands: Vec<(&str, &[&str])> = steps.iter().zip(&values).map(|((id, _), v)| (*id, v.as_slice())).collect();
        let sent = self.run_all_at(&carry.at, &commands);
        if let Err(error) = sent {
            log::error!("lok: the carried object did not drop: {error}");
        }
        true
    }

    /// The carried frame and its lines, once it moved.
    pub(super) fn carried(&self) -> Option<&(Rect, Vec<Rect>)> {
        self.carry.as_ref()?.moved.as_ref()
    }

    /// The carried frame and how many lines show (tests).
    #[cfg(any(test, feature = "test-support"))]
    pub fn carried_now(&self) -> Option<(Rect, usize)> {
        self.carried().map(|(frame, lines)| (*frame, lines.len()))
    }
}

/// The slide's snap targets, in twips.
fn targets<'a>(slide: &Slide, others: &'a [Rect], reach: i32) -> snap::Targets<'a> {
    let objects = &slide.objects;
    let on = |kind| objects.snap_guides.then(|| objects.guides.iter().filter(move |g| g.kind == kind));
    let xs = on(crate::show::GuideKind::Vertical).into_iter().flatten().map(|g| twips(g.at.0)).collect();
    let ys = on(crate::show::GuideKind::Horizontal).into_iter().flatten().map(|g| twips(g.at.1)).collect();
    snap::Targets {
        slide: (twips(slide.size.0), twips(slide.size.1)),
        others,
        guides: (xs, ys),
        grid: objects.snap_grid.then(|| (twips(objects.grid.0), twips(objects.grid.1))),
        reach,
    }
}
