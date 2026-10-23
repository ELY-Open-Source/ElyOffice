//! The chosen pages laid on the chosen sheets, as PDF.

use std::collections::BTreeMap;

use anyhow::{Context as _, anyhow, bail};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream, dictionary};

use super::choices::{Choices, Set};
use crate::print::ROUNDING;

/// Catalog entries that name pages no longer there.
const STALE: [&str; 4] = ["Outlines", "OpenAction", "AcroForm", "PageLabels"];

/// A PDF matrix, `[a b c d e f]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Matrix(pub [f64; 6]);

impl Matrix {
    /// This, then `next`.
    fn then(self, next: Matrix) -> Matrix {
        let ([a, b, c, d, e, f], [g, h, i, j, k, l]) = (self.0, next.0);
        Matrix([a * g + b * i, a * h + b * j, c * g + d * i, c * h + d * j, e * g + f * i + k, e * h + f * j + l])
    }

    pub(super) fn apply(self, (x, y): (f64, f64)) -> (f64, f64) {
        let [a, b, c, d, e, f] = self.0;
        (a * x + c * y + e, b * x + d * y + f)
    }
}

/// A page's box, unit and turn.
pub(super) struct Shown {
    corners: [f64; 4],
    unit: f64,
    turn: i64,
}

impl Shown {
    pub(super) fn of(document: &Document, page: ObjectId) -> anyhow::Result<Self> {
        let corners = |bounds: &Object| -> anyhow::Result<[f64; 4]> {
            let numbers = bounds.as_array()?.iter().map(|n| Ok(f64::from(n.as_float()?))).collect::<anyhow::Result<Vec<_>>>()?;
            let [x0, y0, x1, y1] = numbers[..] else {
                bail!("a page box has {} numbers", numbers.len());
            };
            Ok([x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)])
        };
        let media = corners(inherited(document, page, b"MediaBox")?.ok_or_else(|| anyhow!("a page has no media box"))?)?;
        // A crop box is cut to the media box.
        let [x0, y0, x1, y1] = match inherited(document, page, b"CropBox")? {
            Some(crop) => {
                let crop = corners(crop)?;
                [crop[0].max(media[0]), crop[1].max(media[1]), crop[2].min(media[2]), crop[3].min(media[3])]
            }
            None => media,
        };
        if x1 <= x0 || y1 <= y0 {
            bail!("a page's crop box lies outside its media box");
        }
        // UserUnit is the page's own, never inherited.
        let unit = match document.get_dictionary(page)?.get(b"UserUnit") {
            Ok(unit) => f64::from(document.dereference(unit)?.1.as_float()?),
            Err(_) => 1.0,
        };
        let turn = inherited(document, page, b"Rotate")?.map_or(Ok(0), Object::as_i64)?.rem_euclid(360);
        if !(unit > 0.0 && turn % 90 == 0) {
            bail!("a page has a unit of {unit} or a turn of {turn}");
        }
        Ok(Self { corners: [x0, y0, x1, y1], unit, turn })
    }

    /// Width and height in points, as it shows.
    pub(super) fn size(&self) -> (f64, f64) {
        let [x0, y0, x1, y1] = self.corners;
        let (width, height) = ((x1 - x0) * self.unit, (y1 - y0) * self.unit);
        // A page turned a quarter shows its other side up.
        if self.turn % 180 == 90 { (height, width) } else { (width, height) }
    }

    /// From the page's own space to its shown points.
    fn matrix(&self) -> Matrix {
        let [x0, y0, x1, y1] = self.corners;
        let (w, h) = ((x1 - x0) * self.unit, (y1 - y0) * self.unit);
        let unit = Matrix([self.unit, 0.0, 0.0, self.unit, -x0 * self.unit, -y0 * self.unit]);
        // Rotate turns the page clockwise as shown.
        unit.then(Matrix(match self.turn {
            90 => [0.0, -1.0, 1.0, 0.0, 0.0, w],
            180 => [-1.0, 0.0, 0.0, -1.0, w, h],
            270 => [0.0, 1.0, -1.0, 0.0, h, 0.0],
            _ => [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        }))
    }
}

/// `key` on the page or its nearest tree node.
pub(super) fn inherited<'a>(document: &'a Document, page: ObjectId, key: &[u8]) -> anyhow::Result<Option<&'a Object>> {
    let mut node = document.get_dictionary(page)?;
    // A loop in the tree must not hang.
    for _ in 0..64 {
        if let Ok(value) = node.get(key) {
            return Ok(Some(document.dereference(value)?.1));
        }
        let Ok(parent) = node.get(b"Parent") else {
            return Ok(None);
        };
        node = document.dereference(parent)?.1.as_dict()?;
    }
    bail!("the page tree is deeper than 64")
}

/// The PDF at `pdf`, laid out as `choices` asks.
pub(super) fn compose(pdf: &std::path::Path, choices: &Choices) -> anyhow::Result<Vec<u8>> {
    let mut document = Document::load(pdf)?;
    let pages: Vec<ObjectId> = document.get_pages().into_values().collect();
    let picked = picked(choices, pages.len())?;
    let mut forms = BTreeMap::new();
    for &n in &picked {
        if let std::collections::btree_map::Entry::Vacant(slot) = forms.entry(n) {
            slot.insert(form(&mut document, pages[n])?);
        }
    }
    let shown = picked.iter().map(|&n| Shown::of(&document, pages[n])).collect::<anyhow::Result<Vec<_>>>()?;
    let laid = lay(choices, &picked, &shown)?;
    let tree = document.new_object_id();
    let (width, height) = choices.sheet;
    let mut sheets = Vec::with_capacity(laid.len());
    for placed in &laid {
        let mut drawn = String::new();
        let mut objects = Dictionary::new();
        for &(n, Matrix([a, b, c, d, e, f])) in placed {
            drawn.push_str(&format!("q {a:.4} {b:.4} {c:.4} {d:.4} {e:.4} {f:.4} cm /P{n} Do Q\n"));
            objects.set(format!("P{n}"), forms[&n]);
        }
        let mut stream = Stream::new(Dictionary::new(), drawn.into_bytes());
        stream.compress()?;
        let contents = document.add_object(stream);
        let media: Vec<Object> = vec![0.into(), 0.into(), (width as f32).into(), (height as f32).into()];
        let mut sheet = dictionary! {
            "Type" => "Page",
            "Parent" => tree,
            "MediaBox" => media,
            "Resources" => dictionary! { "XObject" => objects },
            "Contents" => contents,
        };
        if choices.upside_down {
            sheet.set("Rotate", 180);
        }
        sheets.push(sheet);
    }
    let blank = {
        let media: Vec<Object> = vec![0.into(), 0.into(), (width as f32).into(), (height as f32).into()];
        dictionary! { "Type" => "Page", "Parent" => tree, "MediaBox" => media, "Resources" => dictionary! { "XObject" => Dictionary::new() } }
    };
    let kids: Vec<Object> = arrange(choices, sheets.len())?
        .into_iter()
        .map(|n| document.add_object(n.map_or_else(|| blank.clone(), |n| sheets[n].clone())).into())
        .collect();
    let count = kids.len() as i64;
    document.objects.insert(tree, dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }.into());
    let root = document.trailer.get(b"Root")?.as_reference()?;
    let catalog = document.get_object_mut(root)?.as_dict_mut()?;
    catalog.set("Pages", tree);
    for key in STALE {
        catalog.remove(key.as_bytes());
    }
    document.prune_objects();
    let mut bytes = Vec::new();
    document.save_to(&mut bytes)?;
    Ok(bytes)
}

/// The chosen pages, from zero, in the dialog's order.
fn picked(choices: &Choices, count: usize) -> anyhow::Result<Vec<usize>> {
    let last = count.checked_sub(1).context("the document has no pages")? as u32;
    let picked: Vec<usize> = match &choices.ranges {
        None => (0..count).collect(),
        Some(ranges) => ranges.iter().filter(|(from, _)| *from <= last).flat_map(|&(from, to)| from as usize..=to.min(last) as usize).collect(),
    };
    if picked.is_empty() {
        bail!("none of the chosen pages are in the document");
    }
    Ok(picked)
}

/// Sheets in print order; None is a blank back.
fn arrange(choices: &Choices, sheets: usize) -> anyhow::Result<Vec<Option<usize>>> {
    let order: Vec<usize> = (0..sheets)
        .filter(|n| match choices.set {
            Set::All => true,
            Set::Even => n % 2 == 1,
            Set::Odd => n % 2 == 0,
        })
        .collect();
    if order.is_empty() {
        bail!("the chosen page set leaves no sheet to print");
    }
    // Two-sided: a front and its back move together.
    let side = if choices.duplex { 2 } else { 1 };
    let mut leaves: Vec<Vec<Option<usize>>> = order.chunks(side).map(|leaf| leaf.iter().copied().map(Some).collect()).collect();
    if let Some(last) = leaves.last_mut() {
        last.resize(side, None);
    }
    if choices.reverse {
        leaves.reverse();
    }
    let copies = choices.copies as usize;
    let leaves: Vec<Vec<Option<usize>>> = match choices.collate {
        true => leaves.iter().cloned().cycle().take(leaves.len() * copies).collect(),
        false => leaves.into_iter().flat_map(|leaf| std::iter::repeat_n(leaf, copies)).collect(),
    };
    Ok(leaves.concat())
}

/// Each sheet's pages: which page, and where it goes.
fn lay(choices: &Choices, picked: &[usize], shown: &[Shown]) -> anyhow::Result<Vec<Vec<(usize, Matrix)>>> {
    let (width, height) = choices.sheet;
    if choices.up == 1 {
        return picked
            .iter()
            .zip(shown)
            .map(|(&n, page)| {
                let (w, h) = page.size();
                let k = choices.scale;
                let fits = |across: f64, down: f64| k * across <= width + ROUNDING && k * down <= height + ROUNDING;
                // Placed from the sheet's top left; turned left if needed.
                let place = match (fits(w, h), fits(h, w)) {
                    (true, _) => Matrix([k, 0.0, 0.0, k, 0.0, height - k * h]),
                    (false, true) => Matrix([0.0, k, -k, 0.0, k * h, height - k * w]),
                    (false, false) => bail!(
                        "page {} at {:.0}% is larger than the {:.0} × {:.0} mm paper",
                        n + 1,
                        k * 100.0,
                        width * 25.4 / 72.0,
                        height * 25.4 / 72.0
                    ),
                };
                Ok(vec![(n, page.matrix().then(place))])
            })
            .collect();
    }
    let (cols, rows) = grid(choices.up, choices.sheet, shown);
    let (cell_w, cell_h) = (width / f64::from(cols), height / f64::from(rows));
    let mut sheets = Vec::new();
    for (pages, looks) in picked.chunks(choices.up as usize).zip(shown.chunks(choices.up as usize)) {
        let placed = pages.iter().zip(looks).enumerate().map(|(slot, (&n, page))| {
            let (col, row) = cell(slot as u32, (cols, rows), choices);
            let (w, h) = page.size();
            let k = (cell_w / w).min(cell_h / h) * choices.scale;
            // Centred in its cell.
            let left = f64::from(col) * cell_w + (cell_w - k * w) / 2.0;
            let bottom = height - f64::from(row + 1) * cell_h + (cell_h - k * h) / 2.0;
            (n, page.matrix().then(Matrix([k, 0.0, 0.0, k, left, bottom])))
        });
        sheets.push(placed.collect());
    }
    Ok(sheets)
}

/// Columns and rows for `up` pages; the largest fit best.
fn grid(up: u32, (width, height): (f64, f64), shown: &[Shown]) -> (u32, u32) {
    let (w, h) = shown.iter().map(Shown::size).fold((0.0, 0.0), |(a, b), (w, h)| (w.max(a), h.max(b)));
    let fit = |(cols, rows): (u32, u32)| (width / f64::from(cols) / w).min(height / f64::from(rows) / h);
    let grids = (1..=up).filter(|&cols| up.is_multiple_of(cols)).map(|cols| (cols, up / cols));
    grids.fold((up, 1), |best, grid| if fit(grid) > fit(best) { grid } else { best })
}

/// Slot `n`'s column and row, by the dialog's n-up order.
fn cell(n: u32, (cols, rows): (u32, u32), choices: &Choices) -> (u32, u32) {
    let order = choices.order;
    let (col, row) = if order.across_first { (n % cols, n / cols) } else { (n / rows, n % rows) };
    (if order.leftward { cols - 1 - col } else { col }, if order.upward { rows - 1 - row } else { row })
}

/// The page as a form, printable annotations drawn on top.
fn form(document: &mut Document, page: ObjectId) -> anyhow::Result<ObjectId> {
    let mut drawn = b"q\n".to_vec();
    for content in document.get_page_contents(page) {
        drawn.extend(document.get_object(content)?.as_stream()?.decompressed_content()?);
        drawn.push(b'\n');
    }
    drawn.extend(b"Q\n");
    let mut resources = match inherited(document, page, b"Resources")? {
        Some(resources) => resources.as_dict()?.clone(),
        None => Dictionary::new(),
    };
    let mut objects = match resources.get(b"XObject") {
        Ok(objects) => document.dereference(objects)?.1.as_dict()?.clone(),
        Err(_) => Dictionary::new(),
    };
    let mut n = 0;
    for (look, place) in appearances(document, page)? {
        // Names the page already uses stay its own.
        while objects.has(format!("ElyAnnot{n}").as_bytes()) {
            n += 1;
        }
        let [a, b, c, d, e, f] = place.0;
        drawn.extend(format!("q {a:.4} {b:.4} {c:.4} {d:.4} {e:.4} {f:.4} cm /ElyAnnot{n} Do Q\n").into_bytes());
        objects.set(format!("ElyAnnot{n}"), look);
        let look = document.get_object_mut(look)?.as_stream_mut()?;
        if look.dict.get(b"Subtype").is_err() {
            look.dict.set("Subtype", "Form");
        }
    }
    resources.set("XObject", objects);
    let [x0, y0, x1, y1] = Shown::of(document, page)?.corners;
    let bounds: Vec<Object> = [x0, y0, x1, y1].into_iter().map(|n| (n as f32).into()).collect();
    let mut dict = dictionary! { "Type" => "XObject", "Subtype" => "Form", "BBox" => bounds, "Resources" => resources };
    if let Some(group) = inherited(document, page, b"Group")? {
        dict.set("Group", group.clone());
    }
    let mut stream = Stream::new(dict, drawn);
    stream.compress()?;
    Ok(document.add_object(stream))
}

/// Printable annotations' looks, and where each goes on the page.
fn appearances(document: &Document, page: ObjectId) -> anyhow::Result<Vec<(ObjectId, Matrix)>> {
    let Ok(annots) = document.get_dictionary(page)?.get(b"Annots") else {
        return Ok(Vec::new());
    };
    let turn = Shown::of(document, page)?.turn;
    let mut looks = Vec::new();
    for annot in document.dereference(annots)?.1.as_array()? {
        let annot = document.dereference(annot)?.1.as_dict()?;
        let flags = annot.get(b"F").and_then(Object::as_i64).unwrap_or(0);
        // Hidden, or not marked for print: left out.
        if flags & 2 != 0 || flags & 4 == 0 {
            continue;
        }
        let Some(look) = appearance(document, annot)? else {
            continue;
        };
        let stream = document.get_object(look)?.as_stream()?;
        let numbers = |object: &Object| -> anyhow::Result<Vec<f64>> {
            document.dereference(object)?.1.as_array()?.iter().map(|n| Ok(f64::from(n.as_float()?))).collect()
        };
        let bounds = numbers(stream.dict.get(b"BBox")?)?;
        let rect = numbers(annot.get(b"Rect")?)?;
        let own = match stream.dict.get(b"Matrix") {
            Ok(matrix) => Matrix(numbers(matrix)?.try_into().map_err(|_| anyhow!("an appearance matrix is not six numbers"))?),
            Err(_) => Matrix([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
        };
        let ([bx0, by0, bx1, by1], [rx0, ry0, rx1, ry1]) = (four(&bounds)?, four(&rect)?);
        // The look's box, through its matrix, fills the rectangle.
        let corners = [(bx0, by0), (bx1, by0), (bx0, by1), (bx1, by1)].map(|corner| own.apply(corner));
        let (lx0, ly0) = corners.iter().fold((f64::MAX, f64::MAX), |(x, y), &(a, b)| (x.min(a), y.min(b)));
        let (lx1, ly1) = corners.iter().fold((f64::MIN, f64::MIN), |(x, y), &(a, b)| (x.max(a), y.max(b)));
        if lx1 - lx0 <= 0.0 || ly1 - ly0 <= 0.0 {
            continue;
        }
        let (sx, sy) = ((rx1 - rx0).abs() / (lx1 - lx0), (ry1 - ry0).abs() / (ly1 - ly0));
        let mut place = Matrix([sx, 0.0, 0.0, sy, rx0.min(rx1) - sx * lx0, ry0.min(ry1) - sy * ly0]);
        // NoRotate: upright as shown, turned about its top left.
        if flags & 16 != 0 {
            place = place.then(unturned(turn, (rx0.min(rx1), ry0.max(ry1))));
        }
        looks.push((look, place));
    }
    Ok(looks)
}

/// Turns back a page's `turn` about `(x, y)`.
fn unturned(turn: i64, (x, y): (f64, f64)) -> Matrix {
    let (cos, sin) = match turn {
        90 => (0.0, 1.0),
        180 => (-1.0, 0.0),
        270 => (0.0, -1.0),
        _ => (1.0, 0.0),
    };
    Matrix([cos, sin, -sin, cos, x - cos * x + sin * y, y - sin * x - cos * y])
}

fn four(numbers: &[f64]) -> anyhow::Result<[f64; 4]> {
    numbers.try_into().map_err(|_| anyhow!("an annotation's box is not four numbers"))
}

/// An annotation's normal look in its state; None draws nothing.
fn appearance(document: &Document, annot: &Dictionary) -> anyhow::Result<Option<ObjectId>> {
    let subtype = annot.get(b"Subtype")?.as_name()?;
    let normal = match annot.get(b"AP") {
        Ok(looks) => document.dereference(looks)?.1.as_dict()?.get(b"N").ok(),
        Err(_) => None,
    };
    let Some(normal) = normal else {
        // Links and popups have no look of their own.
        if matches!(subtype, b"Link" | b"Popup") {
            return Ok(None);
        }
        bail!("a printable {} annotation has no appearance", String::from_utf8_lossy(subtype));
    };
    if let Ok(id) = normal.as_reference()
        && document.get_object(id)?.as_stream().is_ok()
    {
        return Ok(Some(id));
    }
    // A look per state: the one /AS names, if any.
    let states = document.dereference(normal)?.1.as_dict()?;
    let state = annot.get(b"AS").and_then(Object::as_name).context("an annotation with states names none")?;
    Ok(states.get(state).ok().map(Object::as_reference).transpose()?)
}

#[cfg(test)]
mod tests;
