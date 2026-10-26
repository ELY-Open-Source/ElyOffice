//! A sheet's row and column geometry from `.uno:SheetGeometryData`.

use std::io;

mod analysis;
pub mod csv;
mod data;
mod page;
mod text;
pub use analysis::*;
pub use data::*;
pub use page::*;

/// The query for sizes and hidden flags of both axes.
pub const GEOMETRY: &str = ".uno:SheetGeometryData?columns=true&rows=true&sizes=true&hidden=true&filtered=false&groups=false";

/// One axis: runs of equal size, to each last index.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Axis {
    /// (last index, size in twips); hidden runs are 0.
    runs: Vec<(u32, i32)>,
}

/// Both axes of the shown sheet.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Geometry {
    pub rows: Axis,
    pub columns: Axis,
}

impl Axis {
    /// How many rows or columns.
    pub fn count(&self) -> u32 {
        self.runs.last().map_or(0, |&(last, _)| last + 1)
    }

    /// The size of `index` in twips; 0 when hidden.
    pub fn size(&self, index: u32) -> i32 {
        self.runs
            .iter()
            .find(|&&(last, _)| index <= last)
            .map_or(0, |&(_, size)| size)
    }

    /// Where `index` starts, in twips.
    pub fn start(&self, index: u32) -> i32 {
        let (mut at, mut first) = (0i64, 0u32);
        for &(last, size) in &self.runs {
            if index <= last {
                return clamp(at + i64::from(index - first) * i64::from(size));
            }
            at += i64::from(last - first + 1) * i64::from(size);
            first = last + 1;
        }
        clamp(at)
    }

    /// The index under `twips`; the last one past the end.
    pub fn at(&self, twips: i32) -> u32 {
        let (mut at, mut first) = (0i64, 0u32);
        let twips = i64::from(twips.max(0));
        for &(last, size) in &self.runs {
            let span = i64::from(last - first + 1) * i64::from(size);
            if size > 0 && twips < at + span {
                let into = u32::try_from((twips - at) / i64::from(size)).expect("inside a run");
                return first + into;
            }
            at += span;
            first = last + 1;
        }
        self.count().saturating_sub(1)
    }
}

impl Axis {
    /// Where `index` starts in tile pixels.
    fn tile_start(&self, index: u32, ppt: f64) -> i64 {
        let (mut at, mut first) = (0i64, 0u32);
        for &(last, size) in &self.runs {
            let pixels = i64::from(to_pixels(size, ppt));
            if index <= last {
                return at + i64::from(index - first) * pixels;
            }
            at += i64::from(last - first + 1) * pixels;
            first = last + 1;
        }
        at
    }

    /// A print-twips position in tile twips at `ppt`.
    pub fn tile(&self, print: i32, ppt: f64) -> i32 {
        let end = self.start(self.count());
        if print >= end {
            return clamp(twips_of(self.tile_start(self.count(), ppt), ppt) + i64::from(print - end));
        }
        let index = self.at(print);
        let (from, size) = (self.start(index), self.size(index));
        let start = twips_of(self.tile_start(index, ppt), ppt);
        let span = twips_of(self.tile_start(index + 1, ppt), ppt) - start;
        clamp(start + i64::from(print - from) * span / i64::from(size))
    }

    /// The print-twips position under tile twips `tile`.
    pub fn print(&self, tile: i32, ppt: f64) -> i32 {
        let pixel = (f64::from(tile.max(0)) * ppt) as i64;
        let (mut at, mut first) = (0i64, 0u32);
        for &(last, size) in &self.runs {
            let pixels = i64::from(to_pixels(size, ppt));
            let span = i64::from(last - first + 1) * pixels;
            if pixels > 0 && pixel < at + span {
                let index = first + u32::try_from((pixel - at) / pixels).expect("inside a run");
                let start = twips_of(self.tile_start(index, ppt), ppt);
                let width = twips_of(self.tile_start(index + 1, ppt), ppt) - start;
                let into = i64::from(tile) - start;
                return clamp(i64::from(self.start(index)) + into * i64::from(size) / width.max(1));
            }
            at += span;
            first = last + 1;
        }
        let end = twips_of(self.tile_start(self.count(), ppt), ppt);
        clamp(i64::from(self.start(self.count())) + i64::from(tile) - end)
    }
}

/// The kernel's `ScViewData::ToPixel`: truncates, never to 0.
fn to_pixels(twips: i32, ppt: f64) -> i32 {
    let pixels = (f64::from(twips) * ppt) as i32;
    if pixels == 0 && twips != 0 { 1 } else { pixels }
}

fn twips_of(pixels: i64, ppt: f64) -> i64 {
    (pixels as f64 / ppt).round() as i64
}

fn clamp(twips: i64) -> i32 {
    i32::try_from(twips).unwrap_or(i32::MAX)
}

/// Parses the kernel's geometry JSON.
pub fn geometry(json: &str) -> io::Result<Geometry> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| io::Error::other(format!("bad sheet geometry: {error}")))?;
    let axis = |name: &str| -> io::Result<Axis> {
        let field = |key: &str| {
            value[name][key]
                .as_str()
                .ok_or_else(|| io::Error::other(format!("no {name} {key} in the geometry")))
        };
        merge(&runs(field("sizes")?)?, &flags(field("hidden")?)?)
    };
    Ok(Geometry {
        rows: axis("rows")?,
        columns: axis("columns")?,
    })
}

/// "value:last value:last " as (last, value) runs.
fn runs(text: &str) -> io::Result<Vec<(u32, i32)>> {
    text.split_whitespace()
        .map(|run| {
            let bad = || io::Error::other(format!("a bad geometry run: {run}"));
            let (value, last) = run.split_once(':').ok_or_else(bad)?;
            Ok((
                last.parse().map_err(|_| bad())?,
                value.parse().map_err(|_| bad())?,
            ))
        })
        .collect()
}

/// "value:last last last ": each later run flips the value.
fn flags(text: &str) -> io::Result<Vec<(u32, i32)>> {
    let mut parts = text.split_whitespace();
    let bad = |part: &str| io::Error::other(format!("a bad flag run: {part}"));
    let head = parts.next().ok_or_else(|| bad(text))?;
    let (value, last) = head.split_once(':').ok_or_else(|| bad(head))?;
    let mut value: i32 = value.parse().map_err(|_| bad(head))?;
    let mut list = vec![(last.parse().map_err(|_| bad(head))?, value)];
    for part in parts {
        value = 1 - value;
        list.push((part.parse().map_err(|_| bad(part))?, value));
    }
    Ok(list)
}

/// Sizes with hidden runs set to 0.
fn merge(sizes: &[(u32, i32)], hidden: &[(u32, i32)]) -> io::Result<Axis> {
    let mut runs: Vec<(u32, i32)> = Vec::new();
    let (mut size, mut hide) = (sizes.iter().peekable(), hidden.iter().peekable());
    let mut first = 0u32;
    while let (Some(&&(size_last, width)), Some(&&(hide_last, hidden))) = (size.peek(), hide.peek())
    {
        let last = size_last.min(hide_last);
        let value = if hidden != 0 { 0 } else { width };
        match runs.last_mut() {
            Some((end, run)) if *run == value => *end = last,
            _ => runs.push((last, value)),
        }
        first = last + 1;
        if size_last == last {
            size.next();
        }
        if hide_last == last {
            hide.next();
        }
    }
    if size.peek().is_some() || hide.peek().is_some() {
        return Err(io::Error::other(format!(
            "geometry runs disagree past index {first}"
        )));
    }
    Ok(Axis { runs })
}

/// What refers to a sheet from elsewhere.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct References {
    /// Formula cells on other sheets.
    pub formulas: u64,
    /// Named ranges.
    pub names: u64,
    /// The first such cell, as "$U.A1"; empty when none.
    pub first: String,
    /// Formulas and names whose targets show only at run time.
    pub dynamic: u64,
}

/// The query for references to sheet `sheet`, from 0.
pub fn references_query(sheet: usize) -> String {
    format!(".uno:SheetReferences?sheet={sheet}")
}

/// Parses `.uno:SheetReferences`.
pub fn references(json: &str) -> io::Result<References> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| io::Error::other(format!("bad sheet references: {error}")))?;
    let number = |key: &str| {
        value[key]
            .as_u64()
            .ok_or_else(|| io::Error::other(format!("no {key} in {value}")))
    };
    Ok(References {
        formulas: number("formulas")?,
        names: number("names")?,
        first: value["first"]
            .as_str()
            .ok_or_else(|| io::Error::other(format!("no first in {value}")))?
            .to_string(),
        dynamic: number("dynamic")?,
    })
}

/// The workbook's named ranges (TABLE-06).
pub const NAMES: &str = ".uno:DefinedNames";

/// One named range.
#[derive(Clone, Debug, PartialEq)]
pub struct Name {
    pub name: String,
    /// The sheet it belongs to; empty for the workbook.
    pub scope: String,
    /// What it refers to, as a formula shows it.
    pub formula: String,
    /// The absolute range it names; empty for a formula.
    pub range: String,
    /// It lost a reference or holds an error.
    pub broken: bool,
}

/// Parses `.uno:DefinedNames`.
pub fn names(json: &str) -> io::Result<Vec<Name>> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| io::Error::other(format!("bad names: {error}")))?;
    let bad = || io::Error::other(format!("unreadable names: {value}"));
    value["names"]
        .as_array()
        .ok_or_else(bad)?
        .iter()
        .map(|entry| {
            let text = |key: &str| entry[key].as_str().map(str::to_string).ok_or_else(bad);
            Ok(Name {
                name: text("name")?,
                scope: text("scope")?,
                formula: text("formula")?,
                range: text("range")?,
                broken: entry["broken"].as_bool().ok_or_else(bad)?,
            })
        })
        .collect()
}

/// The selection as one absolute range (TABLE-06).
pub const SELECTED: &str = ".uno:SelectedRange";

/// The selection as one range, and the data inside it.
#[derive(Clone, Debug, PartialEq)]
pub struct Selection {
    /// Absolute, with the sheet.
    pub range: String,
    /// First column, row, last column, row; None with no data.
    pub area: Option<(u32, u32, u32, u32)>,
    /// The area as an absolute range; empty with no data.
    pub data: String,
    /// The area's first row.
    pub top: Vec<String>,
    /// The kernel takes the first row for headers.
    pub header: bool,
    /// Cells in the range holding anything.
    pub filled: u64,
}

/// Parses `.uno:SelectedRange`; None when several ranges are selected.
pub fn selected_range(json: &str) -> io::Result<Option<Selection>> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| io::Error::other(format!("bad selection: {error}")))?;
    let bad = || io::Error::other(format!("unreadable selection: {value}"));
    let range = value["range"].as_str().ok_or_else(bad)?;
    if range.is_empty() {
        return Ok(None);
    }
    let filled = value["filled"].as_u64().ok_or_else(bad)?;
    let Some(area) = value["area"].as_array() else {
        return Ok(Some(Selection {
            range: range.to_string(),
            area: None,
            data: String::new(),
            top: Vec::new(),
            header: false,
            filled,
        }));
    };
    let corner = |at: usize| -> io::Result<u32> {
        area.get(at)
            .and_then(|n| n.as_str()?.parse().ok())
            .ok_or_else(bad)
    };
    let top = value["top"]
        .as_array()
        .ok_or_else(bad)?
        .iter()
        .map(|text| text.as_str().map(str::to_string).ok_or_else(bad))
        .collect::<io::Result<_>>()?;
    Ok(Some(Selection {
        range: range.to_string(),
        area: Some((corner(0)?, corner(1)?, corner(2)?, corner(3)?)),
        data: value["data"].as_str().ok_or_else(bad)?.to_string(),
        top,
        header: value["header"].as_bool().ok_or_else(bad)?,
        filled,
    }))
}

/// The query that checks whether `name` can name a range.
pub fn name_check_query(name: &str) -> String {
    format!("{NAMES}?name={}", encode(name))
}

/// One cell by address, its sheet named.
pub const CELL: &str = ".uno:CellAt";

/// One cell as the kernel holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct CellAt {
    /// Absolute, with the sheet.
    pub address: String,
    /// As shown.
    pub text: String,
    /// As typed: a formula keeps its `=`.
    pub input: String,
}

pub fn cell_at_query(address: &str) -> String {
    format!("{CELL}?address={}", encode(address))
}

/// The cell read; an address naming no cell is refused.
pub fn cell_at(json: &str) -> io::Result<CellAt> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|error| io::Error::other(format!("bad cell: {error}")))?;
    if value["error"] == "address" {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "the address names no cell on a sheet"));
    }
    let field = |key: &str| value[key].as_str().map(str::to_owned).ok_or_else(|| io::Error::other(format!("unreadable cell: {value}")));
    Ok(CellAt { address: field("address")?, text: field("text")?, input: field("input")? })
}

/// `text` percent-encoded for a query value.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (byte as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

/// A name check: "valid", "address" or "characters".
pub fn name_check(json: &str) -> io::Result<&'static str> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| io::Error::other(format!("bad name check: {error}")))?;
    match value["name"].as_str() {
        Some("valid") => Ok("valid"),
        Some("address") => Ok("address"),
        Some("characters") => Ok("characters"),
        _ => Err(io::Error::other(format!("unreadable name check: {value}"))),
    }
}

/// The column's letters: 0 is "A", 26 is "AA".
pub fn column_name(index: u32) -> String {
    let mut name = Vec::new();
    let mut n = index + 1;
    while n > 0 {
        let digit = u8::try_from((n - 1) % 26).expect("a letter");
        name.push(b'A' + digit);
        n = (n - 1) / 26;
    }
    name.reverse();
    String::from_utf8(name).expect("ASCII letters")
}

/// The A1 address of a cell, both from 0.
pub fn address(column: u32, row: u32) -> String {
    format!("{}{}", column_name(column), row + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = r#"{"commandName":".uno:SheetGeometryData","maxtiledcolumn":"1023","maxtiledrow":"500000",
        "columns":{"sizes":"1280:1 2000:2 1280:1023 ","hidden":"0:0 1 1023 "},
        "rows":{"sizes":"256:1048575 ","hidden":"0:1048575 "}}"#;

    #[test]
    fn hidden_columns_fold_and_positions_add_up() {
        let geometry = geometry(JSON).expect("the geometry parses");
        let columns = &geometry.columns;
        assert_eq!(columns.count(), 1024);
        assert_eq!(columns.size(1), 0, "B hides");
        assert_eq!(columns.size(2), 2000);
        assert_eq!(columns.start(2), 1280);
        assert_eq!(columns.start(3), 3280);
        assert_eq!(columns.at(1279), 0);
        assert_eq!(columns.at(1280), 2, "a hidden column takes no room");
        assert_eq!(columns.at(i32::MAX), 1023);
        assert_eq!(geometry.rows.start(10), 2560);
        assert_eq!(geometry.rows.at(2559), 9);
    }

    #[test]
    fn names_and_addresses() {
        assert_eq!(column_name(0), "A");
        assert_eq!(column_name(25), "Z");
        assert_eq!(column_name(26), "AA");
        assert_eq!(column_name(16383), "XFD");
        assert_eq!(address(1, 9), "B10");
    }

    #[test]
    fn broken_runs_fail() {
        assert!(runs("12:x").is_err());
        assert!(flags("0:3 x").is_err());
        assert_eq!(flags("1:2 5").expect("flags"), [(2, 1), (5, 0)]);
        assert!(merge(&[(3, 10)], &[(5, 0)]).is_err(), "lengths must agree");
    }

    #[test]
    fn tiles_round_each_row_to_whole_pixels() {
        // 293 twips at 2/15 px per twip truncate to 39.
        let rows = Axis { runs: vec![(9999, 293)] };
        let ppt = 2.0 / 15.0;
        assert_eq!(to_pixels(293, ppt), 39);
        let row = 4997u32;
        let print = rows.start(row);
        let tile = rows.tile(print, ppt);
        assert_eq!(tile, (f64::from(39 * row) / ppt).round() as i32, "row tops line up");
        assert_eq!(rows.print(tile, ppt), print, "and come back");
        let mid = rows.tile(print + 146, ppt);
        assert_eq!(rows.at(rows.print(mid, ppt)), row, "a point inside stays in its row");
        assert!(tile < print, "rounding down pulls far rows up");
    }

    #[test]
    fn hidden_rows_take_no_tile_space_and_tiny_rows_take_a_pixel() {
        let rows = Axis { runs: vec![(1, 300), (3, 0), (4, 3)] };
        let ppt = 1.0 / 15.0;
        assert_eq!(rows.tile(rows.start(4), ppt), rows.tile(rows.start(2), ppt), "hidden rows fold");
        assert_eq!(to_pixels(3, ppt), 1, "never 0 for a shown row");
    }

}
