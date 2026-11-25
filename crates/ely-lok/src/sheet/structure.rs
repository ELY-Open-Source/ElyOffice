//! Sheets, rows, columns and merges by address (#107).

use std::io;

use super::encode;

pub const SHEETS: &str = ".uno:SheetsAt";
pub const CHECK: &str = ".uno:StructureAt";

/// One sheet, as `sheet list` shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Sheet {
    pub name: String,
    /// Absolute; None for an empty sheet.
    pub used: Option<String>,
    pub hidden: bool,
}

/// Why the kernel turned a structure verb down.
#[derive(Debug, PartialEq)]
pub enum Refused {
    /// No such sheet; the name asked.
    Sheet(String),
    /// The sheet, or the workbook's structure, is protected.
    Protected(String),
    Name,
    Taken,
    /// A position past the last one allowed.
    Position(u64),
    /// The only visible sheet.
    Last(String),
    Address,
    /// Inserting pushes content past the sheet's end.
    Edge,
    Size,
    Single,
    Merged,
    Unmerged,
    /// A number format code that does not parse.
    Number,
}

/// The query checking `verb` before it runs.
pub fn check_query(verb: &str, address: &str, value: &str) -> String {
    format!("{CHECK}?verb={}&address={}&value={}", encode(verb), encode(address), encode(value))
}

/// The range the verb takes, or its refusal.
pub fn checked(json: &str) -> io::Result<Result<String, Refused>> {
    let value = parsed(json)?;
    let bad = || io::Error::other(format!("unreadable structure check: {value}"));
    let sheet = || value["sheet"].as_str().map(str::to_owned).ok_or_else(bad);
    let refused = match value["error"].as_str() {
        None => return value["address"].as_str().map(|address| Ok(address.to_owned())).ok_or_else(bad),
        Some("sheet") => Refused::Sheet(sheet()?),
        Some("protected") => Refused::Protected(sheet()?),
        Some("name") => Refused::Name,
        Some("taken") => Refused::Taken,
        Some("position") => Refused::Position(value["last"].as_u64().ok_or_else(bad)?),
        Some("last") => Refused::Last(sheet()?),
        Some("address") => Refused::Address,
        Some("edge") => Refused::Edge,
        Some("size") => Refused::Size,
        Some("single") => Refused::Single,
        Some("merged") => Refused::Merged,
        Some("unmerged") => Refused::Unmerged,
        Some("number") => Refused::Number,
        Some(other) => return Err(io::Error::other(format!("unknown structure refusal: {other}"))),
    };
    Ok(Err(refused))
}

/// Every sheet, in order.
pub fn sheets(json: &str) -> io::Result<Vec<Sheet>> {
    let value = parsed(json)?;
    let bad = || io::Error::other(format!("unreadable sheet list: {value}"));
    let list = value["sheets"].as_array().ok_or_else(bad)?;
    list.iter()
        .map(|sheet| {
            let used = match &sheet["used"] {
                serde_json::Value::Null => None,
                used => Some(used.as_str().ok_or_else(bad)?.to_owned()),
            };
            let name = sheet["name"].as_str().ok_or_else(bad)?.to_owned();
            Ok(Sheet { name, used, hidden: sheet["hidden"].as_bool().ok_or_else(bad)? })
        })
        .collect()
}

fn parsed(json: &str) -> io::Result<serde_json::Value> {
    serde_json::from_str(json).map_err(|error| io::Error::other(format!("bad structure answer: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_check_encodes_every_part() {
        assert_eq!(check_query("rows.insert", "'A&B'!5:7", ""), ".uno:StructureAt?verb=rows.insert&address=%27A%26B%27%215%3A7&value=");
    }

    #[test]
    fn checks_name_the_range_or_the_refusal() {
        assert_eq!(checked(r#"{"address":"$S.$A$5:$AMJ$7"}"#).expect("readable"), Ok("$S.$A$5:$AMJ$7".into()));
        assert_eq!(checked(r#"{"error":"sheet","sheet":"Nope","last":3}"#).expect("readable"), Err(Refused::Sheet("Nope".into())));
        assert_eq!(checked(r#"{"error":"position","sheet":"New","last":3}"#).expect("readable"), Err(Refused::Position(3)));
        assert_eq!(checked(r#"{"error":"last","sheet":"Only","last":2}"#).expect("readable"), Err(Refused::Last("Only".into())));
        assert_eq!(checked(r#"{"error":"merged","sheet":"S","last":2}"#).expect("readable"), Err(Refused::Merged));
        assert_eq!(checked(r#"{"error":"number","sheet":"S","last":2}"#).expect("readable"), Err(Refused::Number));
        assert!(checked(r#"{"error":"other"}"#).is_err(), "an unknown refusal fails");
        assert!(checked(r#"{"error":"sheet"}"#).is_err(), "a refusal without its sheet fails");
    }

    #[test]
    fn sheets_list_used_areas_and_hidden_ones() {
        let json = r#"{"commandName":".uno:SheetsAt","sheets":[{"name":"A","used":"$A.$A$1:$B$2","hidden":false},{"name":"B","hidden":true}]}"#;
        let want = vec![
            Sheet { name: "A".into(), used: Some("$A.$A$1:$B$2".into()), hidden: false },
            Sheet { name: "B".into(), used: None, hidden: true },
        ];
        assert_eq!(sheets(json).expect("readable"), want);
        assert!(sheets(r#"{"sheets":[{"name":"A","used":3,"hidden":false}]}"#).is_err(), "a used area is text");
    }
}
