//! A slide's objects, selection and guides (SHOW-03 to 06).

use std::io;

use serde_json::Value;

/// The shown slide's objects, selection, guides and snapping.
pub const OBJECTS: &str = ".uno:SlideObjects";

/// One object on the slide, in z order.
#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    /// The name the file gives it; often empty.
    pub name: String,
    /// text, shape, line, image, group, table, chart, media…
    pub kind: String,
    /// title, body or other; empty when it is none.
    pub placeholder: String,
    /// Left, top, width, height in 1/100 mm.
    pub rect: (i64, i64, i64, i64),
    pub locked: bool,
    pub selected: bool,
}

/// A fill or outline color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Paint {
    None,
    Mixed,
    Color(u32),
}

/// The marked objects; `None` means mixed.
#[derive(Clone, Debug, PartialEq)]
pub struct Selection {
    pub count: usize,
    /// As [`Object::kind`], or mixed.
    pub kind: String,
    pub can_group: bool,
    pub can_ungroup: bool,
    /// The one marked object's text is being edited.
    pub editing: bool,
    pub fill: Paint,
    pub line: Paint,
    /// 1/100 mm.
    pub line_width: Option<i64>,
    /// Percent.
    pub transparency: Option<i64>,
    pub shadow: Option<bool>,
    /// off, shrink, grow, or mixed.
    pub autofit: String,
    /// Left, top, width, height in 1/100 mm.
    pub rect: (i64, i64, i64, i64),
    /// 1/100 degree.
    pub rotation: i64,
    pub rotation_center: (i64, i64),
    pub lock_position: Option<bool>,
    pub lock_size: Option<bool>,
}

/// A guide; lines use one coordinate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Guide {
    pub kind: GuideKind,
    /// 1/100 mm on the slide.
    pub at: (i64, i64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GuideKind {
    Horizontal,
    Vertical,
    Point,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Objects {
    pub objects: Vec<Object>,
    pub selection: Option<Selection>,
    pub snap_grid: bool,
    pub snap_guides: bool,
    pub guides: Vec<Guide>,
}

fn int(value: &Value, key: &str) -> io::Result<i64> {
    value[key]
        .as_i64()
        .ok_or_else(|| io::Error::other(format!("no {key} in {value}")))
}

fn flag(value: &Value, key: &str) -> io::Result<bool> {
    value[key]
        .as_bool()
        .ok_or_else(|| io::Error::other(format!("no {key} in {value}")))
}

fn text(value: &Value, key: &str) -> io::Result<String> {
    value[key]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| io::Error::other(format!("no {key} in {value}")))
}

/// A number where `-2` means mixed.
fn maybe(value: &Value, key: &str) -> io::Result<Option<i64>> {
    int(value, key).map(|n| (n != -2).then_some(n))
}

fn rect(value: &Value) -> io::Result<(i64, i64, i64, i64)> {
    Ok((
        int(value, "x")?,
        int(value, "y")?,
        int(value, "width")?,
        int(value, "height")?,
    ))
}

fn paint(value: &Value, key: &str) -> io::Result<Paint> {
    Ok(match int(value, key)? {
        -1 => Paint::None,
        -2 => Paint::Mixed,
        color => Paint::Color(
            u32::try_from(color).map_err(|_| io::Error::other(format!("bad {key} in {value}")))?,
        ),
    })
}

fn selection(value: &Value) -> io::Result<Option<Selection>> {
    if int(value, "count")? == 0 {
        return Ok(None);
    }
    let flag = |key| maybe(value, key).map(|n| n.map(|n| n == 1));
    Ok(Some(Selection {
        count: usize::try_from(int(value, "count")?).map_err(io::Error::other)?,
        kind: text(value, "kind")?,
        can_group: self::flag(value, "canGroup")?,
        can_ungroup: self::flag(value, "canUngroup")?,
        editing: self::flag(value, "editing")?,
        fill: paint(value, "fill")?,
        line: paint(value, "line")?,
        line_width: maybe(value, "lineWidth")?,
        transparency: maybe(value, "transparency")?,
        shadow: flag("shadow")?,
        autofit: text(value, "autofit")?,
        rect: rect(value)?,
        rotation: int(value, "rotation")?,
        rotation_center: (int(value, "rotationX")?, int(value, "rotationY")?),
        lock_position: flag("lockPosition")?,
        lock_size: flag("lockSize")?,
    }))
}

/// Parses `.uno:SlideObjects`.
pub fn objects(json: &str) -> io::Result<Objects> {
    let value: Value = serde_json::from_str(json)
        .map_err(|error| io::Error::other(format!("bad objects: {error}")))?;
    let list = |key: &str| {
        value[key]
            .as_array()
            .ok_or_else(|| io::Error::other(format!("no {key} in {value}")))
    };
    let objects = list("objects")?
        .iter()
        .map(|item| {
            Ok(Object {
                name: text(item, "name")?,
                kind: text(item, "kind")?,
                placeholder: text(item, "placeholder")?,
                rect: rect(item)?,
                locked: flag(item, "locked")?,
                selected: flag(item, "selected")?,
            })
        })
        .collect::<io::Result<_>>()?;
    let guides = list("guides")?
        .iter()
        .map(|item| {
            let kind = match text(item, "kind")?.as_str() {
                "horizontal" => GuideKind::Horizontal,
                "vertical" => GuideKind::Vertical,
                "point" => GuideKind::Point,
                other => return Err(io::Error::other(format!("bad guide kind {other}"))),
            };
            Ok(Guide {
                kind,
                at: (int(item, "x")?, int(item, "y")?),
            })
        })
        .collect::<io::Result<_>>()?;
    Ok(Objects {
        objects,
        selection: selection(&value["selection"])?,
        snap_grid: flag(&value, "snapGrid")?,
        snap_guides: flag(&value, "snapGuides")?,
        guides,
    })
}
