//! A range by address, its sheet named (#107).

use std::io;

use super::encode;

/// The most cells one range read or write takes.
pub const MOST: u64 = 1_000_000;

pub const RANGE: &str = ".uno:RangeAt";

/// One cell: as shown, and as typed.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub text: String,
    /// A formula keeps its `=`.
    pub input: String,
}

/// A range's cells, row by row.
#[derive(Clone, Debug, PartialEq)]
pub struct Range {
    /// Absolute, with the sheet.
    pub address: String,
    pub rows: Vec<Vec<Cell>>,
}

/// Why the kernel turned a range down.
#[derive(Debug, PartialEq)]
pub enum Refused {
    /// No range on a named sheet.
    Address,
    /// Over [`MOST`] cells; the count.
    Size(u64),
    /// A range whose size differs from the data's.
    Shape,
    /// Data past the sheet's last row or column.
    Edge,
}

/// Query reading `address`; a bare sheet is its used area.
pub fn read_query(address: &str) -> String {
    format!("{RANGE}?address={}", encode(address))
}

/// The query naming where `rows` by `columns` at `address` go.
pub fn target_query(address: &str, rows: usize, columns: usize) -> String {
    format!("{RANGE}?address={}&rows={rows}&columns={columns}", encode(address))
}

/// The range read, or the kernel's refusal.
pub fn read(json: &str) -> io::Result<Result<Range, Refused>> {
    let value = parsed(json)?;
    if let Some(refused) = refusal(&value)? {
        return Ok(Err(refused));
    }
    let bad = || io::Error::other(format!("unreadable range: {value}"));
    let text = |cell: &serde_json::Value, key: &str| cell[key].as_str().map(str::to_owned).ok_or_else(bad);
    let rows = value["rows"].as_array().ok_or_else(bad)?;
    let rows = rows
        .iter()
        .map(|row| {
            let cells = row.as_array().ok_or_else(bad)?;
            cells.iter().map(|cell| Ok(Cell { text: text(cell, "text")?, input: text(cell, "input")? })).collect()
        })
        .collect::<io::Result<_>>()?;
    Ok(Ok(Range { address: text(&value, "address")?, rows }))
}

/// The absolute range a write goes to, or the refusal.
pub fn target(json: &str) -> io::Result<Result<String, Refused>> {
    let value = parsed(json)?;
    if let Some(refused) = refusal(&value)? {
        return Ok(Err(refused));
    }
    value["address"].as_str().map(|address| Ok(address.to_owned())).ok_or_else(|| io::Error::other(format!("unreadable range: {value}")))
}

/// Rows as the kernel's write takes them: a JSON array.
pub fn rows_json(rows: &[Vec<String>]) -> String {
    serde_json::Value::from(rows).to_string()
}

fn parsed(json: &str) -> io::Result<serde_json::Value> {
    serde_json::from_str(json).map_err(|error| io::Error::other(format!("bad range: {error}")))
}

fn refusal(value: &serde_json::Value) -> io::Result<Option<Refused>> {
    Ok(match value["error"].as_str() {
        None => None,
        Some("address") => Some(Refused::Address),
        Some("shape") => Some(Refused::Shape),
        Some("edge") => Some(Refused::Edge),
        Some("size") => Some(Refused::Size(value["cells"].as_u64().ok_or_else(|| io::Error::other(format!("unreadable range: {value}")))?)),
        Some(other) => return Err(io::Error::other(format!("unknown range refusal: {other}"))),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_encode_the_address() {
        assert_eq!(read_query("'My Sheet'!A1:B2"), ".uno:RangeAt?address=%27My%20Sheet%27%21A1%3AB2");
        assert_eq!(target_query("S!B2", 3, 4), ".uno:RangeAt?address=S%21B2&rows=3&columns=4");
    }

    #[test]
    fn a_read_keeps_rows_and_refusals_are_named() {
        let json = r#"{"commandName":".uno:RangeAt","address":"$S.$A$1:$B$1","rows":[[{"text":"42","input":"=6*7"},{"text":"","input":""}]]}"#;
        let range = read(json).expect("readable").expect("a range");
        assert_eq!(range.address, "$S.$A$1:$B$1");
        assert_eq!(range.rows, vec![vec![Cell { text: "42".into(), input: "=6*7".into() }, Cell { text: String::new(), input: String::new() }]]);
        assert_eq!(read(r#"{"error":"address"}"#).expect("readable"), Err(Refused::Address));
        assert_eq!(read(r#"{"address":"$S.$A$1:$B$2000000","cells":4000000,"error":"size"}"#).expect("readable"), Err(Refused::Size(4_000_000)));
        assert_eq!(target(r#"{"error":"edge"}"#).expect("readable"), Err(Refused::Edge));
        assert_eq!(target(r#"{"address":"$S.$B$2:$C$3"}"#).expect("readable"), Ok("$S.$B$2:$C$3".into()));
        assert!(read(r#"{"error":"other"}"#).is_err(), "an unknown refusal fails");
        assert!(read(r#"{"rows":[[{"text":"x"}]]}"#).is_err(), "a cell without input fails");
    }

    #[test]
    fn rows_go_as_a_json_array() {
        assert_eq!(rows_json(&[vec!["a\tb".into(), "\"q\"".into()]]), r#"[["a\tb","\"q\""]]"#);
    }
}
