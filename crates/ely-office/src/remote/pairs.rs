//! `PROPERTY=VALUE ...` arguments shared by the `format` commands.

use std::collections::BTreeMap;

const NONE: &str = "no format properties given (use PROPERTY=VALUE)";
const PROPERTY: &str = "{property}: unknown format property";
const PAIR: &str = "{argument}: expected PROPERTY=VALUE";
const TWICE: &str = "{property}: property given more than once";
const SWITCH: &str = "{property}: invalid value \"{value}\" (expected on or off)";
const SIZE: &str = "size: invalid value \"{value}\" (must be a number from 1 to 999)";
const COLOR: &str = "{property}: invalid color \"{value}\" (expected #RRGGBB or none)";

/// Each known property once, with a value on one line.
pub fn parse<'a>(pairs: &[&'a str], known: impl Fn(&str) -> bool) -> Result<BTreeMap<&'a str, &'a str>, String> {
    if pairs.is_empty() {
        return Err(NONE.into());
    }
    let mut given = BTreeMap::new();
    for pair in pairs {
        let bad = || PAIR.replace("{argument}", pair);
        let (key, value) = pair.split_once('=').ok_or_else(bad)?;
        if value.is_empty() || value.contains(['\n', '\r', '\t']) {
            return Err(bad());
        }
        if !known(key) {
            return Err(PROPERTY.replace("{property}", key));
        }
        if given.insert(key, value).is_some() {
            return Err(TWICE.replace("{property}", key));
        }
    }
    Ok(given)
}

/// `template` filled with `key` and `value`.
pub fn said(template: &str, key: &str, value: &str) -> String {
    template.replace("{property}", key).replace("{value}", value)
}

/// `on` or `off`.
pub fn switch(key: &str, value: &str) -> Result<bool, String> {
    match value {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(said(SWITCH, key, value)),
    }
}

/// A size of 1 to 999 points, in twips.
pub fn twips(value: &str) -> Result<u32, String> {
    match value.parse::<f64>() {
        // In range, so the cast keeps the value.
        Ok(points) if (1.0..=999.0).contains(&points) => Ok((points * 20.0).round() as u32),
        _ => Err(said(SIZE, "size", value)),
    }
}

/// `#RRGGBB` as its number; None for `none`.
pub fn rgb(key: &str, value: &str) -> Result<Option<u32>, String> {
    if value == "none" {
        return Ok(None);
    }
    let hex = value.strip_prefix('#').filter(|hex| hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit()));
    hex.and_then(|hex| u32::from_str_radix(hex, 16).ok()).map(Some).ok_or_else(|| said(COLOR, key, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairs_are_known_single_and_on_one_line() {
        let known = |key: &str| ["bold", "font"].contains(&key);
        let got = parse(&["bold=on", "font=Noto Sans=x"], known).expect("fine");
        assert_eq!(got.into_iter().collect::<Vec<_>>(), [("bold", "on"), ("font", "Noto Sans=x")]);
        assert_eq!(parse(&[], known), Err(NONE.into()));
        assert_eq!(parse(&["bold"], known), Err("bold: expected PROPERTY=VALUE".into()));
        assert_eq!(parse(&["bold="], known), Err("bold=: expected PROPERTY=VALUE".into()));
        assert_eq!(parse(&["font=a\tb"], known), Err("font=a\tb: expected PROPERTY=VALUE".into()));
        assert_eq!(parse(&["wrap=on"], known), Err("wrap: unknown format property".into()));
        assert_eq!(parse(&["bold=on", "bold=off"], known), Err("bold: property given more than once".into()));
    }

    #[test]
    fn values_convert_or_say_why_not() {
        assert_eq!(switch("bold", "on"), Ok(true));
        assert_eq!(switch("bold", "off"), Ok(false));
        assert_eq!(switch("bold", "yes"), Err("bold: invalid value \"yes\" (expected on or off)".into()));
        assert_eq!(twips("12"), Ok(240));
        assert_eq!(twips("10.5"), Ok(210));
        assert_eq!((twips("1"), twips("999")), (Ok(20), Ok(19980)), "both ends");
        assert!(twips("0.5").is_err() && twips("1000").is_err() && twips("big").is_err());
        assert_eq!(rgb("color", "#B00020"), Ok(Some(0xB00020)));
        assert_eq!(rgb("color", "none"), Ok(None));
        assert_eq!(rgb("color", "red"), Err("color: invalid color \"red\" (expected #RRGGBB or none)".into()));
        assert!(rgb("color", "#12345").is_err() && rgb("color", "#12345G").is_err());
    }
}
