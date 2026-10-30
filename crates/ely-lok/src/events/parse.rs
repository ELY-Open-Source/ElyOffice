//! Callback payload parsers.

use serde_json::Value;

use super::Rect;

/// The focused paragraph or cell for assistive tech (§7 tail).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct A11y {
    pub text: String,
    /// Caret and selection in UTF-16 units; LOK's -1 is None.
    pub caret: Option<usize>,
    pub selection: Option<(usize, usize)>,
}

/// A kernel window event that matters to the guard.
pub enum Dialog {
    /// A dialog is up; close it.
    Opened { window: u32, title: Option<String> },
    /// A window got its title.
    Titled { window: u32, title: String },
}

/// A payload that is neither a rectangle nor empty.
#[derive(Debug, PartialEq)]
pub struct Unreadable;

/// "x, y, w, h" and later fields; "EMPTY" is none.
pub fn rect_of(text: &str) -> Result<Option<Rect>, Unreadable> {
    let text = text.trim();
    if text.is_empty() || text.starts_with("EMPTY") {
        return Ok(None);
    }
    let mut parts = text.split(',').map(|part| part.trim().parse::<i32>());
    let mut next = || parts.next().and_then(Result::ok).ok_or(Unreadable);
    Ok(Some((next()?, next()?, next()?, next()?)))
}

/// A print-twips anchor, when the kernel gives one.
pub type Anchor = Option<(i32, i32)>;

/// The visible text cursor, with its optional print-twips anchor.
pub fn cursor_of(text: &str) -> Result<(Option<Rect>, Anchor), Unreadable> {
    if !text.trim_start().starts_with('{') {
        return Ok((rect_of(text)?, None));
    }
    let json: Value = serde_json::from_str(text).map_err(|_| Unreadable)?;
    if json.get("relrect").is_some() || json.get("refpoint").is_some() {
        let rect = rect_of(json["relrect"].as_str().ok_or(Unreadable)?)?.ok_or(Unreadable)?;
        let anchor = point_of(json["refpoint"].as_str().ok_or(Unreadable)?)?;
        Ok((Some(rect), Some(anchor)))
    } else {
        Ok((
            rect_of(json["rectangle"].as_str().ok_or(Unreadable)?)?,
            None,
        ))
    }
}

/// Text selection rectangles sharing an optional print-twips anchor.
pub fn selection_of(text: &str) -> Result<(Vec<Rect>, Anchor), Unreadable> {
    let (text, anchor) = match text.split_once("::") {
        Some((rects, point)) => (rects, Some(point_of(point)?)),
        None => (text, None),
    };
    let rects = text
        .split(';')
        .map(rect_of)
        .collect::<Result<Vec<_>, _>>()?;
    Ok((rects.into_iter().flatten().collect(), anchor))
}

fn point_of(text: &str) -> Result<(i32, i32), Unreadable> {
    let mut parts = text.split(',');
    let mut next = || {
        parts
            .next()
            .ok_or(Unreadable)?
            .trim()
            .parse()
            .map_err(|_| Unreadable)
    };
    let point = (next()?, next()?);
    if parts.next().is_some() {
        return Err(Unreadable);
    }
    Ok(point)
}

/// A search's matches, each by its first rectangle.
pub fn found_of(text: &str) -> Option<Vec<Rect>> {
    let json: Value = serde_json::from_str(text).ok()?;
    json["searchResultSelection"]
        .as_array()?
        .iter()
        .map(|found| rect_of(found["rectangles"].as_str()?.split(';').next()?).ok()?)
        .collect()
}

/// The missing font families of a FONTS_MISSING payload.
pub fn fonts_of(text: &str) -> Option<Vec<String>> {
    let json: Value = serde_json::from_str(text).ok()?;
    json["fontsmissing"]
        .as_array()?
        .iter()
        .map(|font| font.as_str().map(str::to_string))
        .collect()
}

/// A STATE_CHANGED payload: ".uno:X=v" or JSON with commandName.
pub fn state_of(text: &str) -> Option<(String, String)> {
    if text.starts_with('{') {
        let json: Value = serde_json::from_str(text).ok()?;
        let name = json["commandName"].as_str()?.to_string();
        if json["dontcare"] == true {
            return Some((name, super::MIXED.to_string()));
        }
        let state = match &json["state"] {
            Value::String(state) => state.clone(),
            other => other.to_string(),
        };
        return Some((name, state));
    }
    let (name, state) = text.split_once('=')?;
    Some((name.to_string(), state.to_string()))
}

/// The text Calc's input line shows, from its JSDialog.
pub fn formula_text(text: &str) -> Option<String> {
    let json: Value = serde_json::from_str(text).ok()?;
    let data = &json["data"];
    let line = json["jsontype"] == "formulabar"
        && data["control_id"] == "sc_input_window"
        && data["action_type"] == "setText";
    line.then(|| data["text"].as_str().map(str::to_string))?
}

/// A ruler's custom tab stops; style 4 is default.
pub fn tabs_of(text: &str) -> Vec<i32> {
    let Some(tabs) = text.split("\"tabs\"").nth(1) else {
        return Vec::new();
    };
    // The keys repeat, so read the pairs as text.
    let number = |field: &str, from: &str| -> Option<i32> {
        let rest = from.split(field).nth(1)?;
        let digits: String = rest
            .trim_start_matches([':', ' ', '"'])
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '-')
            .collect();
        digits.parse().ok()
    };
    tabs.split('{')
        .skip(2)
        .filter_map(|entry| Some((number("\"position\"", entry)?, number("\"style\"", entry)?)))
        .filter(|(_, style)| *style != 4)
        .map(|(position, _)| position)
        .collect()
}

/// A completion payload: command name and success.
pub fn result_of(text: &str) -> Option<(String, bool)> {
    let json: Value = serde_json::from_str(text).ok()?;
    Some((
        json["commandName"].as_str()?.to_string(),
        json["success"].as_bool()?,
    ))
}

/// A number field sent as text or number.
fn number(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
}

/// The dialog event in a WINDOW or JSDIALOG payload.
pub fn dialog_of(jsdialog: bool, text: &str) -> Option<Dialog> {
    let json: Value = serde_json::from_str(text).ok()?;
    let action = json["action"].as_str().unwrap_or("");
    let window = u32::try_from(number(&json["id"])?).ok()?;
    let title = json["title"]
        .as_str()
        .or_else(|| json["text"].as_str())
        .map(str::to_string);
    if !jsdialog && action == "title_changed" {
        return Some(Dialog::Titled {
            window,
            title: title?,
        });
    }
    let opens = if jsdialog {
        json["jsontype"] == "dialog" && action != "close"
    } else {
        // A tooltip is no dialog.
        action == "created" && json["type"] != "tooltip"
    };
    opens.then_some(Dialog::Opened { window, title })
}

/// A signed LOK position; -1 and absent are None.
fn position(value: &Value) -> Option<usize> {
    let signed = value
        .as_i64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))?;
    usize::try_from(signed).ok()
}

/// Folds a focus, caret or selection payload into `a11y`.
fn fold_paragraph(a11y: &mut A11y, json: &Value) {
    if let Some(text) = json["content"].as_str() {
        a11y.text = text.to_string();
    }
    if !json["position"].is_null() {
        a11y.caret = position(&json["position"]);
    }
    if !json["start"].is_null() {
        a11y.selection = position(&json["start"]).zip(position(&json["end"]));
    }
}

/// Folds an A11Y callback of LOK kind `kind` (62..69).
pub fn a11y_update(a11y: &mut A11y, kind: i32, text: &str) {
    let Ok(json) = serde_json::from_str::<Value>(text) else {
        log::warn!("lok: an a11y payload is not JSON");
        return;
    };
    match kind {
        // Focused cell, editing state: a nested paragraph.
        67 | 68 => fold_paragraph(a11y, &json["paragraph"]),
        // Object or cell selection.
        69 => {
            *a11y = A11y::default();
            if json["action"] != "remove" {
                a11y.text = json["text"].as_str().unwrap_or_default().to_string();
            }
        }
        _ => fold_paragraph(a11y, &json),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_formula_selection_keeps_its_shared_reference_point() {
        assert_eq!(
            selection_of("402, 0, 157, 292::6373, 380"),
            Ok((vec![(402, 0, 157, 292)], Some((6373, 380))))
        );
        assert_eq!(
            selection_of("1, 2, 3, 4; 5, 6, 7, 8:: 5491, 380"),
            Ok((vec![(1, 2, 3, 4), (5, 6, 7, 8)], Some((5491, 380))))
        );
    }

    #[test]
    fn a_formula_cursor_uses_relative_coordinates() {
        let payload = r#"{"viewId":"0","rectangle":"6690, 28045, 0, 292","relrect":"317, 15, 0, 292","refpoint":"6373, 29160"}"#;
        assert_eq!(
            cursor_of(payload),
            Ok((Some((317, 15, 0, 292)), Some((6373, 29160))))
        );
    }

    #[test]
    fn ordinary_text_geometry_stays_absolute() {
        assert_eq!(cursor_of("1, 2, 0, 4"), Ok((Some((1, 2, 0, 4)), None)));
        assert_eq!(
            cursor_of(r#"{"rectangle":"1, 2, 0, 4","viewId":"0"}"#),
            Ok((Some((1, 2, 0, 4)), None))
        );
        assert_eq!(
            selection_of("1, 2, 3, 4; 5, 6, 7, 8"),
            Ok((vec![(1, 2, 3, 4), (5, 6, 7, 8)], None))
        );
        assert_eq!(cursor_of("EMPTY"), Ok((None, None)));
        assert_eq!(selection_of(""), Ok((Vec::new(), None)));
    }

    #[test]
    fn a_bad_reference_point_is_an_error() {
        for point in ["", "1", "x, 2", "1, 2, 3", "2147483648, 2"] {
            assert_eq!(
                selection_of(&format!("1, 2, 3, 4::{point}")),
                Err(Unreadable)
            );
        }
        for payload in [
            r#"{"rectangle":"1,2,0,4","relrect":"1,2,0,4"}"#,
            r#"{"rectangle":"1,2,0,4","refpoint":"1,2"}"#,
            r#"{"rectangle":"1,2,0,4","relrect":"EMPTY","refpoint":"1,2"}"#,
        ] {
            assert_eq!(cursor_of(payload), Err(Unreadable));
        }
    }

    #[test]
    fn a_rectangle_reads_strictly() {
        assert_eq!(rect_of("1, 2, 3, 4, 0, 5"), Ok(Some((1, 2, 3, 4))));
        assert_eq!(rect_of("EMPTY, 0, 0"), Ok(None));
        assert_eq!(rect_of(""), Ok(None));
        assert_eq!(rect_of(r#"{"x": 1}, 2, 3, 4, 5"#), Err(Unreadable));
        assert_eq!(rect_of("1, 2, 3"), Err(Unreadable));
    }

    #[test]
    fn dont_care_states_read_as_mixed() {
        let mixed = state_of(r#"{"commandName":".uno:Bold","dontcare":true}"#);
        assert_eq!(
            mixed,
            Some((".uno:Bold".into(), super::super::MIXED.into()))
        );
        assert_eq!(
            state_of(".uno:Bold=false"),
            Some((".uno:Bold".into(), "false".into()))
        );
    }

    #[test]
    fn ruler_tabs_keep_the_custom_stops() {
        let ruler = r#"{ "margin1": 0, "tabs": { "": { "position": 1251, "style": 4}, "": { "position": 2000, "style": 0}, "": { "position": 3500, "style": 3} }, "unit": "\""}"#;
        assert_eq!(tabs_of(ruler), [2000, 3500]);
        assert_eq!(tabs_of(r#"{"tabs": { }}"#), Vec::<i32>::new());
    }

    #[test]
    fn states_parse_both_forms() {
        assert_eq!(
            state_of(".uno:Bold=true"),
            Some((".uno:Bold".into(), "true".into()))
        );
        assert_eq!(
            state_of(r#"{"commandName":".uno:FontHeight","state":"12"}"#),
            Some((".uno:FontHeight".into(), "12".into()))
        );
        assert_eq!(state_of("garbage"), None);
    }

    fn opened(event: Option<Dialog>) -> Option<(u32, Option<String>)> {
        match event? {
            Dialog::Opened { window, title } => Some((window, title)),
            Dialog::Titled { .. } => None,
        }
    }

    #[test]
    fn dialogs_open_on_created_or_jsdialog() {
        let created = dialog_of(false, r#"{"id":"7","action":"created","title":"Font"}"#);
        assert_eq!(opened(created), Some((7, Some("Font".into()))));
        assert!(dialog_of(false, r#"{"id":"7","action":"invalidate"}"#).is_none());
        let tip = r#"{"id":"2","action":"created","type":"tooltip","parentId":"0"}"#;
        assert!(dialog_of(false, tip).is_none(), "a tooltip is no dialog");
        let titled = dialog_of(
            false,
            r#"{"id":"2","action":"title_changed","title":"Delete"}"#,
        );
        assert!(matches!(titled, Some(Dialog::Titled { window: 2, .. })));
        let js = r#"{"id":"Delete","text":"Delete Contents","id":2,"jsontype":"dialog"}"#;
        assert_eq!(
            opened(dialog_of(true, js)),
            Some((2, Some("Delete Contents".into())))
        );
        let update = dialog_of(true, r#"{"jsontype":"dialog","action":"update","id":2}"#);
        assert_eq!(opened(update), Some((2, None)));
        assert!(dialog_of(true, r#"{"jsontype":"formulabar","id":0}"#).is_none());
        assert!(dialog_of(true, r#"{"jsontype":"dialog","action":"close","id":2}"#).is_none());
    }

    #[test]
    fn the_calc_input_line_reads_out() {
        let line = r#"{"jsontype":"formulabar","action":"action","id":0,"data":{"control_id":"sc_input_window","text":"Hello","action_type":"setText"}}"#;
        assert_eq!(formula_text(line).as_deref(), Some("Hello"));
        let address = r#"{"jsontype":"addressinputfield","data":{"control_id":"pos_window","text":"A1","action_type":"setText"}}"#;
        assert_eq!(formula_text(address), None);
    }

    #[test]
    fn a11y_folds_focus_caret_and_selection() {
        let mut a11y = A11y::default();
        let focus = r#"{"content":"Hello","position":"2","start":"1","end":"3"}"#;
        a11y_update(&mut a11y, 62, focus);
        let want = A11y {
            text: "Hello".into(),
            caret: Some(2),
            selection: Some((1, 3)),
        };
        assert_eq!(a11y, want);
        a11y_update(&mut a11y, 63, r#"{"position":"5"}"#);
        assert_eq!(a11y.caret, Some(5));
        a11y_update(&mut a11y, 64, r#"{"start":"-1","end":"-1"}"#);
        assert_eq!(a11y.selection, None, "-1 clears the selection");
        let cell = r#"{"paragraph":{"content":"B1","position":"0"}}"#;
        a11y_update(&mut a11y, 67, cell);
        assert_eq!(a11y.text, "B1");
        a11y_update(
            &mut a11y,
            69,
            r#"{"cell":"1","action":"create","text":"A1 text"}"#,
        );
        assert_eq!(a11y.text, "A1 text");
        a11y_update(
            &mut a11y,
            69,
            r#"{"cell":"1","action":"remove","text":"A1 text"}"#,
        );
        assert_eq!(a11y, A11y::default(), "a removed selection clears");
    }
}
