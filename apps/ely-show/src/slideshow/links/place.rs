//! Where a shape sits on its slide, as xmloff writes.

use std::io;

use quick_xml::events::BytesStart;

use super::{Rect, attr, bad};

/// Placed by their two end points, never transformed.
const ENDS: [&str; 3] = ["draw:line", "draw:connector", "draw:measure"];

/// Whether `name` is placed by its two end points.
pub(super) fn by_ends(name: &str) -> bool {
    ENDS.contains(&name)
}

/// Affine: a, b, c, d, then shift e, f.
type Matrix = [f64; 6];

/// A length such as `2.5cm` in 1/100 mm.
pub(super) fn length(text: &str) -> io::Result<i64> {
    let at = text
        .find(|c: char| c.is_alphabetic())
        .ok_or_else(|| bad(format!("a length with no unit: {text}")))?;
    let (number, unit) = text.split_at(at);
    let number: f64 = number.parse().map_err(|_| bad(format!("a bad length: {text}")))?;
    let mm = match unit {
        "cm" => 10.0,
        "mm" => 1.0,
        "in" => 25.4,
        "pt" => 25.4 / 72.0,
        "pc" => 25.4 / 6.0,
        "px" => 25.4 / 96.0,
        _ => return Err(bad(format!("a length in {unit}: {text}"))),
    };
    Ok((number * mm * 100.0).round() as i64)
}

fn lengths<const N: usize>(tag: &BytesStart, keys: [&str; N]) -> io::Result<[Option<i64>; N]> {
    let mut out = [None; N];
    for (part, key) in out.iter_mut().zip(keys) {
        *part = attr(tag, key)?.map(|text| length(&text)).transpose()?;
    }
    Ok(out)
}

/// The box a shape covers on its slide, if given.
pub(super) fn bounds(name: &str, tag: &BytesStart) -> io::Result<Option<Rect>> {
    if by_ends(name) {
        return Ok(match lengths(tag, ["svg:x1", "svg:y1", "svg:x2", "svg:y2"])? {
            [Some(x1), Some(y1), Some(x2), Some(y2)] => Some((x1.min(x2), y1.min(y2), (x2 - x1).abs(), (y2 - y1).abs())),
            _ => None,
        });
    }
    let [x, y, width, height] = lengths(tag, ["svg:x", "svg:y", "svg:width", "svg:height"])?;
    let (Some(width), Some(height)) = (width, height) else {
        return Ok(None);
    };
    let Some(transform) = attr(tag, "draw:transform")? else {
        return Ok(x.zip(y).map(|(x, y)| (x, y, width, height)));
    };
    // The unit square, sized, placed, then transformed (xmloff's import).
    let (x, y) = (x.unwrap_or(0) as f64, y.unwrap_or(0) as f64);
    let mut matrix = [width as f64, 0.0, 0.0, height as f64, x, y];
    for op in transforms(&transform)? {
        matrix = then(matrix, op);
    }
    let corners = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)].map(|(u, v)| {
        let [a, b, c, d, e, f] = matrix;
        (a * u + c * v + e, b * u + d * v + f)
    });
    let (xs, ys) = (corners.map(|p| p.0), corners.map(|p| p.1));
    let low = |v: [f64; 4]| v.into_iter().fold(f64::INFINITY, f64::min).round();
    let high = |v: [f64; 4]| v.into_iter().fold(f64::NEG_INFINITY, f64::max).round();
    let (left, top, right, bottom) = (low(xs), low(ys), high(xs), high(ys));
    Ok(Some((left as i64, top as i64, (right - left) as i64, (bottom - top) as i64)))
}

/// `op` applied after `matrix`.
fn then(m: Matrix, op: Matrix) -> Matrix {
    let [a, b, c, d, e, f] = op;
    [
        a * m[0] + c * m[1],
        b * m[0] + d * m[1],
        a * m[2] + c * m[3],
        b * m[2] + d * m[3],
        a * m[4] + c * m[5] + e,
        b * m[4] + d * m[5] + f,
    ]
}

/// `draw:transform`'s list, in the order it applies.
fn transforms(text: &str) -> io::Result<Vec<Matrix>> {
    let mut out = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        let (name, after) = rest.split_once('(').ok_or_else(|| bad(format!("a bad transform: {text}")))?;
        let (inside, after) = after.split_once(')').ok_or_else(|| bad(format!("a bad transform: {text}")))?;
        rest = after.trim_start_matches(|c: char| c.is_whitespace() || c == ',');
        let args: Vec<&str> = inside.split(|c: char| c.is_whitespace() || c == ',').filter(|arg| !arg.is_empty()).collect();
        let number = |at: usize| -> io::Result<f64> {
            let arg = args.get(at).ok_or_else(|| bad(format!("a transform short of values: {text}")))?;
            arg.parse().map_err(|_| bad(format!("a bad transform value: {text}")))
        };
        let shift = |at: usize| -> io::Result<f64> {
            let arg = args.get(at).ok_or_else(|| bad(format!("a transform short of values: {text}")))?;
            length(arg).map(|n| n as f64)
        };
        // As xmloff: bare radians, rotate and skew mirrored (#i78696#).
        out.push(match name.trim() {
            "rotate" => {
                let (sin, cos) = (-number(0)?).sin_cos();
                [cos, sin, -sin, cos, 0.0, 0.0]
            }
            "skewX" => [1.0, 0.0, -number(0)?.tan(), 1.0, 0.0, 0.0],
            "skewY" => [1.0, -number(0)?.tan(), 0.0, 1.0, 0.0, 0.0],
            "scale" => {
                let x = number(0)?;
                let y = if args.len() > 1 { number(1)? } else { x };
                [x, 0.0, 0.0, y, 0.0, 0.0]
            }
            "translate" => [1.0, 0.0, 0.0, 1.0, shift(0)?, if args.len() > 1 { shift(1)? } else { 0.0 }],
            "matrix" => [number(0)?, number(1)?, number(2)?, number(3)?, shift(4)?, shift(5)?],
            other => return Err(bad(format!("a transform {other}: {text}"))),
        });
    }
    Ok(out)
}
