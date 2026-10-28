//! Formats, filters, rules and links read from the kernel.

use std::io;

/// The current cell's format code and kind.
pub const NUMBER: &str = ".uno:NumberFormatCode";
/// A filter column's entries.
pub const FILTER: &str = ".uno:AutoFilterValues";
/// A cell's choices: its validation list.
pub const LIST: &str = ".uno:DataSelectValues";
/// The current cell's validation.
pub const VALIDATION: &str = ".uno:Validation";
/// The shown sheet's conditional formats.
pub const FORMATS: &str = ".uno:ConditionalFormats";
/// Sources whose data shows as last saved (§9.3).
pub const LINKS: &str = ".uno:ExternalLinks";
/// Cells showing the file's result: their function is unknown.
pub const KEPT: &str = ".uno:KeptResults";

fn parse(json: &str, what: &str) -> io::Result<serde_json::Value> {
    serde_json::from_str(json).map_err(|error| io::Error::other(format!("bad {what}: {error}")))
}

fn text(value: &serde_json::Value, key: &str) -> io::Result<String> {
    value[key]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| io::Error::other(format!("no {key} in {value}")))
}

fn flag(value: &serde_json::Value, key: &str) -> io::Result<bool> {
    value[key]
        .as_bool()
        .ok_or_else(|| io::Error::other(format!("no {key} in {value}")))
}

/// The query that checks a format code.
pub fn number_check_query(code: &str) -> String {
    format!("{NUMBER}?code={}", super::encode(code))
}

/// Parses a code check.
pub fn number_valid(json: &str) -> io::Result<bool> {
    flag(&parse(json, "format check")?, "valid")
}

/// Parses `.uno:NumberFormatCode`: the code and its kind.
pub fn number_format(json: &str) -> io::Result<(String, String)> {
    let value = parse(json, "number format")?;
    Ok((text(&value, "code")?, text(&value, "type")?))
}

/// The query for a cell's filter entries or choices.
pub fn cell_query(base: &str, column: u32, row: u32) -> String {
    format!("{base}?col={column}&row={row}")
}

/// One filter entry, as the kernel's own menu shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub text: String,
    /// Other columns' filters hide its rows.
    pub hidden: bool,
    /// The column's own filter keeps it.
    pub selected: bool,
}

/// A filter column: whether it filters, and its entries.
#[derive(Clone, Debug, PartialEq)]
pub struct Filter {
    pub filtered: bool,
    pub entries: Vec<Entry>,
}

/// Parses `.uno:AutoFilterValues`.
pub fn filter_values(json: &str) -> io::Result<Filter> {
    let value = parse(json, "filter values")?;
    let entries = value["values"]
        .as_array()
        .ok_or_else(|| io::Error::other(format!("no values in {value}")))?
        .iter()
        .map(|entry| {
            Ok(Entry {
                text: text(entry, "text")?,
                hidden: flag(entry, "hidden")?,
                selected: flag(entry, "selected")?,
            })
        })
        .collect::<io::Result<_>>()?;
    Ok(Filter {
        filtered: flag(&value, "filtered")?,
        entries,
    })
}

/// Parses `.uno:ExternalLinks`.
pub fn links(json: &str) -> io::Result<Vec<String>> {
    strings(json, "links")
}

/// Parses `.uno:KeptResults`: addresses with their sheet.
pub fn kept(json: &str) -> io::Result<Vec<String>> {
    strings(json, "cells")
}

/// The array of strings at `key`.
fn strings(json: &str, key: &str) -> io::Result<Vec<String>> {
    let value = parse(json, key)?;
    value[key]
        .as_array()
        .ok_or_else(|| io::Error::other(format!("no {key} in {value}")))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| io::Error::other(format!("a bad entry in {key}: {item}")))
        })
        .collect()
}

/// Parses `.uno:DataSelectValues`.
pub fn list_values(json: &str) -> io::Result<Vec<String>> {
    let value = parse(json, "choices")?;
    value["values"]
        .as_array()
        .ok_or_else(|| io::Error::other(format!("no values in {value}")))?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| io::Error::other(format!("a bad choice: {entry}")))
        })
        .collect()
}

/// A validation as the kernel reports and takes it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Validation {
    /// any, whole, decimal, date, time, textlength, list, custom.
    pub kind: String,
    pub operator: String,
    pub value1: String,
    pub value2: String,
    /// List entries to set; the kernel quotes them.
    pub list: Vec<String>,
    pub dropdown: bool,
    pub ignore_blank: bool,
    pub show_input: bool,
    pub input_title: String,
    pub input_message: String,
    pub show_error: bool,
    pub error_title: String,
    pub error_message: String,
    /// stop, warning or info.
    pub error_style: String,
}

impl Validation {
    /// The rule `rules.validation` takes.
    pub fn to_json(&self) -> String {
        serde_json::json!({
            "type": self.kind,
            "operator": self.operator,
            "value1": self.value1,
            "value2": self.value2,
            "list": self.list,
            "dropdown": self.dropdown,
            "ignoreBlank": self.ignore_blank,
            "showInput": self.show_input,
            "inputTitle": self.input_title,
            "inputMessage": self.input_message,
            "showError": self.show_error,
            "errorTitle": self.error_title,
            "errorMessage": self.error_message,
            "errorStyle": self.error_style,
        })
        .to_string()
    }
}

/// Parses `.uno:Validation`.
pub fn validation(json: &str) -> io::Result<Validation> {
    let value = parse(json, "validation")?;
    let kind = text(&value, "type")?;
    // A cell without a rule reports its type alone.
    if value.get("operator").is_none() {
        return Ok(Validation {
            kind,
            ..Validation::default()
        });
    }
    Ok(Validation {
        kind,
        operator: text(&value, "operator")?,
        value1: text(&value, "value1")?,
        value2: text(&value, "value2")?,
        list: Vec::new(),
        dropdown: flag(&value, "dropdown")?,
        ignore_blank: flag(&value, "ignoreBlank")?,
        show_input: flag(&value, "showInput")?,
        input_title: text(&value, "inputTitle")?,
        input_message: text(&value, "inputMessage")?,
        show_error: flag(&value, "showError")?,
        error_title: text(&value, "errorTitle")?,
        error_message: text(&value, "errorMessage")?,
        error_style: text(&value, "errorStyle")?,
    })
}

/// One conditional format on the shown sheet.
#[derive(Clone, Debug, PartialEq)]
pub struct Format {
    pub key: u64,
    pub range: String,
    /// cell, scale, databar, iconset or other.
    pub kind: String,
    /// The cell style a cell rule applies.
    pub style: String,
    pub value: String,
}

/// Parses `.uno:ConditionalFormats`.
pub fn formats(json: &str) -> io::Result<Vec<Format>> {
    let value = parse(json, "conditional formats")?;
    value["formats"]
        .as_array()
        .ok_or_else(|| io::Error::other(format!("no formats in {value}")))?
        .iter()
        .map(|entry| {
            let cell = text(entry, "kind")? == "cell";
            let optional = |key| {
                if cell {
                    text(entry, key)
                } else {
                    Ok(String::new())
                }
            };
            Ok(Format {
                key: entry["key"]
                    .as_u64()
                    .ok_or_else(|| io::Error::other(format!("no key in {entry}")))?,
                range: text(entry, "range")?,
                kind: text(entry, "kind")?,
                style: optional("style")?,
                value: optional("value1")?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_reads_parse() {
        let filter = r#"{"filtered":true,"values":[{"text":"a","hidden":false,"selected":true},{"text":"","hidden":true,"selected":false}]}"#;
        let filter = filter_values(filter).expect("entries");
        assert!(filter.filtered && filter.entries[1].hidden && !filter.entries[1].selected);
        assert_eq!(
            list_values(r#"{"values":["Red","Green"]}"#).expect("list"),
            ["Red", "Green"]
        );
        assert_eq!(validation(r#"{"type":"any"}"#).expect("none").kind, "any");
        let help = r#"{"type":"any","operator":"equal","value1":"","value2":"","dropdown":true,"ignoreBlank":true,"showInput":true,"inputTitle":"","inputMessage":"An id","showError":false,"errorTitle":"","errorMessage":"","errorStyle":"stop"}"#;
        assert_eq!(validation(help).expect("help").input_message, "An id");
        assert!(
            validation(r#"{"type":"list","operator":"equal","dropdown":true}"#).is_err(),
            "a part is missing"
        );
        let formats = formats(
            r#"{"formats":[{"key":1,"range":"A1:A3","kind":"cell","style":"Good","value1":"5"},{"key":2,"range":"B1","kind":"scale"}]}"#,
        )
        .expect("formats");
        assert_eq!(
            (formats[0].style.as_str(), formats[1].kind.as_str()),
            ("Good", "scale")
        );
        assert!(
            number_format(r#"{"code":"0.00"}"#).is_err(),
            "a missing kind fails"
        );
        assert_eq!(
            number_check_query("0.0 %"),
            ".uno:NumberFormatCode?code=0.0%20%25"
        );
    }
}
