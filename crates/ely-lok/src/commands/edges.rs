//! A border edge: `keep`, `none`, or `style:width:color` in 1/100 mm.

use std::io;

use super::invalid;

/// The kernel's `BorderLineStyle` numbers it draws.
const EDGE_STYLES: &[i64] = &[0, 1, 2, 3, 14, 16, 17];

/// The widest border line, in 1/100 mm.
const EDGE_WIDTH_MAX: i64 = 200;

/// One edge's line; `None` for `keep`, `none` clears.
pub(super) fn edge_json(name: &str, value: &str) -> io::Result<Option<serde_json::Value>> {
    if matches!(value, "keep" | "none") {
        return Ok(None);
    }
    let bad = || invalid(format!("{name} is not keep, none or style:width:color: {value}"));
    let mut parts = value.split(':').map(|part| part.parse::<i64>().map_err(|_| bad()));
    let (Some(style), Some(width), Some(color), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(bad());
    };
    let (style, width, color) = (style?, width?, color?);
    if !EDGE_STYLES.contains(&style) {
        return Err(invalid(format!("{name} has no line style {style}")));
    }
    if !(1..=EDGE_WIDTH_MAX).contains(&width) {
        return Err(invalid(format!("{name} width is out of 1..{EDGE_WIDTH_MAX}: {width}")));
    }
    if !(0..=0xFF_FFFF).contains(&color) {
        return Err(invalid(format!("{name} color is out of range: {color}")));
    }
    Ok(Some(serde_json::json!({
        "type": "[]any",
        "value": line_parts([color, 0, width, 0, style, width]),
    })))
}

/// Color, inner, outer, distance, style, total width.
fn line_parts(parts: [i64; 6]) -> Vec<serde_json::Value> {
    parts
        .iter()
        .map(|part| serde_json::json!({ "type": "long", "value": part.to_string() }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::edge_json;
    use crate::commands::find;

    #[test]
    fn border_edges_take_a_style_width_and_color() {
        let edge = |value| edge_json("Top", value);
        assert!(edge("keep").expect("keep").is_none(), "an edge left alone");
        let dotted = edge("1:35:16711680").expect("dotted").expect("a line");
        assert_eq!(dotted["value"][0]["value"], "16711680");
        assert_eq!(dotted["value"][4]["value"], "1");
        assert_eq!(dotted["value"][5]["value"], "35");
        assert!(edge("none").expect("none").is_none(), "an absent line clears");
        for bad in ["", "9:18:0", "0:0:0", "0:18", "0:18:0:1", "0:18:x", "0:18:16777216"] {
            assert!(edge(bad).is_err(), "{bad:?}");
        }
        let json = find("cells.border")
            .expect("mapped")
            .args_json(&["0:18:0", "keep", "none", "keep", "keep", "keep"])
            .expect("six edges")
            .expect("json");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("json");
        assert!(parsed["OuterBorder.LeftBorder"].is_object());
        assert!(parsed.get("OuterBorder.RightBorder").is_none());
        assert!(parsed.get("OuterBorder.TopBorder").is_none());
        // Left and top may change: 0x04 | 0x01.
        assert_eq!(parsed["InnerBorder.ValidFlags"]["value"], "5");
    }
}
