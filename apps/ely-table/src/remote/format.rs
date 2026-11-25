//! `format FILE ADDRESS PROPERTY=VALUE ...`: cell formats by address (#107).

use std::collections::BTreeMap;

use ely_office::remote::{self, pairs};

const CHOICE: &str = "{property}: invalid value \"{value}\" (expected {choices})";
pub(super) const NUMBER: &str = "number: invalid number format code \"{value}\"";
const BORDER: &str = "{property}: invalid border \"{value}\" (all|outer|none; side: none or style:width:#RRGGBB)";

const SWITCHES: [&str; 4] = ["bold", "italic", "underline", "wrap"];
const EDGES: [&str; 6] = ["border-left", "border-top", "border-right", "border-bottom", "border-inner-h", "border-inner-v"];
/// Line styles: name and the kernel's `SvxBorderLineStyle`.
const STYLES: [(&str, u8); 7] = [("solid", 0), ("dotted", 1), ("dashed", 2), ("double", 3), ("fine-dashed", 14), ("dash-dot", 16), ("dash-dot-dot", 17)];
/// Twips per unit a line width may be written in.
const WIDTHS: [(&str, f64); 2] = [("pt", 20.0), ("mm", 1440.0 / 25.4)];
/// The widest line, 2 mm, in twips.
const WIDEST: f64 = 2.0 * 1440.0 / 25.4;
/// `border=all|outer`'s line: solid, 0.75 pt, black.
const THIN: &str = "0:15:0";
const ALIGNS: &str = "left|center|right|justify|general";
const VALIGNS: &str = "top|middle|bottom|standard";

/// The words sent for `format FILE ADDRESS PROPERTY=VALUE ...`.
pub(super) fn request(words: &[&str]) -> Result<Vec<String>, String> {
    let [path, address, pairs @ ..] = words else {
        return Err("unknown command".into());
    };
    Ok(vec!["range.format".into(), remote::absolute(path)?, (*address).into(), kernel_lines(pairs)?])
}

/// The user's number code within the kernel's lines.
pub(super) fn number_code(lines: &str) -> &str {
    lines.lines().find_map(|line| line.strip_prefix("number=")).unwrap_or_default()
}

/// `pairs` checked, as key=value lines the kernel takes.
fn kernel_lines(words: &[&str]) -> Result<String, String> {
    let given = pairs::parse(words, known)?;
    let mut lines = BTreeMap::new();
    // A named edge overrides what `border` gives it.
    if let Some(value) = given.get("border") {
        for (edge, line) in border(value)? {
            lines.insert(edge.to_owned(), line.to_owned());
        }
    }
    for (key, value) in given.into_iter().filter(|(key, _)| *key != "border") {
        lines.insert(key.to_owned(), converted(key, value)?);
    }
    Ok(lines.into_iter().map(|(key, value)| format!("{key}={value}\n")).collect())
}

fn known(key: &str) -> bool {
    SWITCHES.contains(&key) || EDGES.contains(&key) || ["font", "size", "color", "background", "number", "align", "valign", "border"].contains(&key)
}

/// `border=all|outer|none` as its edges.
fn border(value: &str) -> Result<Vec<(&'static str, &'static str)>, String> {
    let (edges, line): (&[&str], _) = match value {
        "all" => (&EDGES, THIN),
        "outer" => (&EDGES[..4], THIN),
        "none" => (&EDGES, "none"),
        _ => return Err(BORDER.replace("{property}", "border").replace("{value}", value)),
    };
    Ok(edges.iter().map(|edge| (*edge, line)).collect())
}

/// One property's value as the kernel takes it.
fn converted(key: &str, value: &str) -> Result<String, String> {
    let said = |template: &str| pairs::said(template, key, value);
    match key {
        _ if SWITCHES.contains(&key) => pairs::switch(key, value).map(|on| if on { "1" } else { "0" }.into()),
        "size" => pairs::twips(value).map(|twips| twips.to_string()),
        "color" | "background" => pairs::rgb(key, value).map(|rgb| rgb.map_or("none".into(), |rgb| rgb.to_string())),
        "align" => choice(value, ALIGNS).ok_or_else(|| said(CHOICE).replace("{choices}", ALIGNS)),
        "valign" => choice(value, VALIGNS).ok_or_else(|| said(CHOICE).replace("{choices}", VALIGNS)),
        _ if EDGES.contains(&key) => edge(value).ok_or_else(|| said(BORDER)),
        // `font` as written; `number` is checked by the kernel.
        _ => Ok(value.into()),
    }
}

fn choice(value: &str, choices: &str) -> Option<String> {
    choices.split('|').any(|one| one == value).then(|| value.into())
}

/// `none` or `style:width:#RRGGBB` as the kernel's `style:twips:decimal`.
fn edge(value: &str) -> Option<String> {
    if value == "none" {
        return Some(value.into());
    }
    let mut parts = value.split(':');
    let (Some(style), Some(width), Some(rgb), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return None;
    };
    let style = STYLES.iter().find(|(name, _)| *name == style)?.1;
    let (amount, per) = WIDTHS.iter().find_map(|(unit, per)| width.strip_suffix(unit).map(|amount| (amount, per)))?;
    let twips = amount.parse::<f64>().ok().filter(|amount| *amount > 0.0).map(|amount| amount * per)?;
    if twips > WIDEST + 1e-9 {
        return None;
    }
    Some(format!("{style}:{}:{}", twips.round().max(1.0), pairs::rgb("", rgb).ok()??))
}

#[cfg(test)]
mod tests;
