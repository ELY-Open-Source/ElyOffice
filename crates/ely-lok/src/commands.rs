//! UI operations mapped to kernel commands (LOK-02, LOK-12).

use std::io;

mod analysis;
mod calc;
mod data;
mod draw;
mod layout;
mod page;
mod refs;
mod sheet;
mod show;
mod text;
mod tools;

/// One argument: its LOK name and what values it takes.
#[derive(Debug, PartialEq)]
pub enum Arg {
    Text(&'static str),
    /// Text that may be empty.
    Loose(&'static str),
    /// A family the kernel lists under `.uno:CharFontName`.
    Font(&'static str),
    /// Points, 1 to 999, in the kernel's 0.05 steps.
    Points(&'static str),
    /// A whole number of LOK type `kind` within `min..=max`.
    Int(&'static str, &'static str, i64, i64),
    Bool(&'static str),
    /// A fixed border line, solid or none; takes no value.
    Line(&'static str, bool),
}

/// How the worker confirms the kernel did the command.
#[derive(Debug, PartialEq)]
pub enum Check {
    /// The completion result reports success.
    Result,
    /// The kernel's `<uno>Count` history drops by one.
    History,
    /// The command's state then equals the first value.
    Reports,
    /// The undo history gains a step.
    Undoable,
    /// The second value's style family then lists the first.
    Listed,
    /// The selected image then reports crop mode.
    Cropping,
    /// The kernel finished; the outcome shows elsewhere.
    Finished,
}

/// One UI operation: kernel command, arguments, confirmation.
#[derive(Debug, PartialEq)]
pub struct Command {
    pub id: &'static str,
    pub uno: &'static str,
    pub args: &'static [Arg],
    pub check: Check,
}

/// Arguments that count things as a read saw them.
const NUMBERED: &[&str] = &[
    "Index", "nTabNumber", "TabNo", "Part", "StartColumn", "StartRow", "EndColumn", "EndRow",
];

impl Command {
    /// Names something the document numbers, as read.
    pub fn addressed(&self, values: &[&str]) -> bool {
        // An index of -1 names nothing: a new tab stop.
        self.id == "macro.apply" || self.args.iter().zip(values).any(|(arg, value)| matches!(arg, Arg::Int(name, ..) if NUMBERED.contains(name)) && *value != "-1")
    }

    /// This command with `values`, checked now (LOK-02).
    #[cfg(feature = "view")]
    pub(crate) fn owned(&self, values: &[&str]) -> io::Result<(&'static str, Vec<String>)> {
        self.args_json(values)?;
        Ok((self.id, values.iter().map(|value| value.to_string()).collect()))
    }
}

pub(crate) const fn command(
    id: &'static str,
    uno: &'static str,
    args: &'static [Arg],
    check: Check,
) -> Command {
    Command {
        id,
        uno,
        args,
        check,
    }
}

/// The command table, in groups.
pub const COMMANDS: &[&[Command]] = &[
    text::ROWS,
    layout::ROWS,
    refs::ROWS,
    tools::ROWS,
    sheet::ROWS,
    calc::ROWS,
    data::ROWS,
    analysis::ROWS,
    page::ROWS,
    show::ROWS,
    draw::ROWS,
];

/// Every row of the table.
pub fn all() -> impl Iterator<Item = &'static Command> {
    COMMANDS.iter().flat_map(|group| group.iter())
}

/// Kernel lists Ely reads: fonts, styles, undo history.
pub const VALUES: &[&str] = &[
    ".uno:CharFontName",
    ".uno:StyleApply",
    ".uno:Undo",
    ".uno:Redo",
    ".uno:ViewAnnotations",
    ".uno:AcceptTrackedChanges",
    ".uno:Bookmarks",
    ".uno:Formula",
    ".uno:WordCount",
    ".uno:Outline",
    crate::sheet::GEOMETRY,
    crate::sheet::NAMES,
    crate::sheet::SELECTED,
    crate::sheet::VALIDATION,
    crate::sheet::FORMATS,
    crate::sheet::NUMBER,
    crate::sheet::CHART,
    crate::sheet::PIVOT,
    crate::sheet::LAYOUT,
    crate::sheet::PROTECTION,
    crate::sheet::LINKS,
    crate::sheet::KEPT,
    ".uno:ElyMacroSelection",
    crate::show::DESIGN,
    crate::show::OBJECTS,
    crate::show::NOTES,
    crate::show::TRANSITION,
    crate::show::ANIMATIONS,
    crate::show::MEDIA,
    crate::show::SLIDE_SHOW,
    crate::show::SLIDE_CHECK,
];

/// Reads that take `?key=value` arguments.
pub const QUERIES: &[&str] = &[
    ".uno:SheetReferences",
    crate::sheet::NAMES,
    crate::sheet::NUMBER,
    crate::sheet::FILTER,
    crate::sheet::LIST,
    crate::sheet::CELL,
    crate::menu::STATES,
];

/// The kernel's font families, from `.uno:CharFontName`.
pub fn fonts(json: &str) -> io::Result<Vec<String>> {
    let list = parse(json)?;
    let fonts = list["commandValues"]
        .as_object()
        .ok_or_else(|| invalid("the font list has no families".into()))?;
    Ok(fonts.keys().cloned().collect())
}

/// Writer's Western and Asian fonts; empty when mixed (0055).
pub fn char_fonts(json: &str) -> io::Result<Option<[String; 2]>> {
    let fonts = parse(json)?;
    // Past Writer's lookup limit: none read.
    if fonts["unread"] == true {
        return Ok(None);
    }
    let read = |key: &str| {
        fonts[key]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| invalid(format!("the fonts lack {key}: {json}")))
    };
    Ok(Some([read("western")?, read("asian")?]))
}

/// One style family: 1 character, 2 paragraph, 8 page.
pub fn styles(json: &str, family: &str) -> io::Result<Vec<String>> {
    let key = match family {
        "1" => "CharacterStyles",
        "2" => "ParagraphStyles",
        "8" => "PageStyles",
        _ => return Err(invalid(format!("no style family {family}"))),
    };
    let list = parse(json)?;
    list["commandValues"][key]
        .as_array()
        .ok_or_else(|| invalid(format!("the style list has no {key}")))?
        .iter()
        .map(|name| {
            name.as_str()
                .map(str::to_string)
                .ok_or_else(|| invalid(format!("a bad {key} entry: {name}")))
        })
        .collect()
}

fn parse(json: &str) -> io::Result<serde_json::Value> {
    serde_json::from_str(json).map_err(|error| invalid(format!("a bad kernel list: {error}")))
}

/// The command for a UI id.
pub fn find(id: &str) -> io::Result<&'static Command> {
    all()
        .find(|command| command.id == id)
        .ok_or_else(|| invalid(format!("no command {id}")))
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

impl Command {
    /// The LOK argument JSON for `values`, checked by kind.
    pub fn args_json(&self, values: &[&str]) -> io::Result<Option<String>> {
        let (lines, taking): (Vec<&Arg>, Vec<&Arg>) = self
            .args
            .iter()
            .partition(|arg| matches!(arg, Arg::Line(..)));
        if values.len() != taking.len() {
            return Err(invalid(format!(
                "{} takes {} values",
                self.id,
                taking.len()
            )));
        }
        if self.args.is_empty() {
            return Ok(None);
        }
        let mut json = serde_json::Map::new();
        for arg in lines {
            if let Arg::Line(name, solid) = arg {
                json.insert((*name).to_string(), line_json(*solid));
            }
        }
        for (arg, value) in taking.into_iter().zip(values) {
            let (name, kind) = match arg {
                Arg::Text(name) | Arg::Font(name) | Arg::Loose(name) => (name, "string"),
                Arg::Points(name) => (name, "float"),
                Arg::Int(name, kind, _, _) => (name, *kind),
                Arg::Bool(name) => (name, "boolean"),
                Arg::Line(..) => unreachable!("lines take no value"),
            };
            if value.trim().is_empty() && !matches!(arg, Arg::Loose(_)) {
                return Err(invalid(format!("{} needs {name}", self.id)));
            }
            match arg {
                Arg::Points(_) => check_points(name, value)?,
                Arg::Int(_, _, min, max) => {
                    let number: i64 = value
                        .parse()
                        .map_err(|_| invalid(format!("{name} is not a whole number: {value}")))?;
                    if !(*min..=*max).contains(&number) {
                        return Err(invalid(format!("{name} is out of {min}..{max}: {value}")));
                    }
                }
                Arg::Bool(_) if !matches!(*value, "true" | "false") => {
                    return Err(invalid(format!("{name} is not true or false: {value}")));
                }
                _ => {}
            }
            json.insert(
                (*name).to_string(),
                serde_json::json!({ "type": kind, "value": value }),
            );
        }
        Ok(Some(serde_json::Value::Object(json).to_string()))
    }
}

/// A border line as LOK takes it: color, widths, style.
fn line_json(solid: bool) -> serde_json::Value {
    let width = if solid { "18" } else { "0" };
    let parts = ["0", "0", width, "0", "0", width];
    let value: Vec<_> = parts
        .iter()
        .map(|part| serde_json::json!({ "type": "long", "value": part }))
        .collect();
    serde_json::json!({ "type": "[]any", "value": value })
}

/// A plain decimal in 0.05 steps from 1 to 999.
fn check_points(name: &str, value: &str) -> io::Result<()> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, "0"));
    let digits = |text: &str| text.bytes().all(|byte| byte.is_ascii_digit());
    if whole.is_empty()
        || whole.len() > 3
        || !digits(whole)
        || fraction.is_empty()
        || fraction.len() > 2
        || !digits(fraction)
    {
        return Err(invalid(format!("{name} is not a plain decimal: {value}")));
    }
    let hundredths: u32 = format!("{whole}{fraction:0<2}")
        .parse()
        .map_err(|_| invalid(format!("{name} is not a number: {value}")))?;
    if !(100..=99_900).contains(&hundredths) {
        return Err(invalid(format!("{name} is out of 1..999: {value}")));
    }
    if !hundredths.is_multiple_of(5) {
        return Err(invalid(format!("{name} takes 0.05 steps: {value}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_real_number_needs_the_read_state() {
        let tab = find("paragraph.tab").expect("the tab stop row");
        assert!(!tab.addressed(&["-1", "1134", "false"]), "a new stop names nothing");
        assert!(tab.addressed(&["0", "0", "true"]), "removing stop 0 does");
        assert!(find("sheet.delete").expect("the row").addressed(&["1"]));
        assert!(!find("cell.enter").expect("the row").addressed(&["1"]));
        assert!(find("notes.set").expect("the row").addressed(&["text", "1"]), "a slide by number");
        // Clicks renumber no row or column: never stale.
        assert!(!find("select.row").expect("the row").addressed(&["3", "0"]), "a header row");
        assert!(!find("columns.width").expect("the row").addressed(&["500", "2"]), "a column's width");
        assert!(!find("rows.height_selected").expect("the row").addressed(&["500"]), "a size names nothing");
    }

    #[test]
    fn ids_are_unique_and_reported_names_too() {
        let rows: Vec<&Command> = all().collect();
        for (i, a) in rows.iter().enumerate() {
            for b in &rows[i + 1..] {
                assert_ne!(a.id, b.id);
                if a.check == Check::Reports || b.check == Check::Reports {
                    assert_ne!(a.uno, b.uno, "a reported state names one row");
                }
            }
        }
    }

    #[test]
    fn border_lines_take_no_values() {
        let all = find("table.borders_all").expect("mapped");
        let json = all.args_json(&[]).expect("no values").expect("lines");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("json");
        assert_eq!(parsed["OuterBorder.LeftBorder"]["type"], "[]any");
        assert_eq!(parsed["InnerBorder.Vertical"]["value"][2]["value"], "18");
        let none = find("table.borders_none").expect("mapped");
        let json = none.args_json(&[]).expect("no values").expect("lines");
        assert!(json.contains("\"value\":\"0\""), "none is width 0");
        assert!(all.args_json(&["x"]).is_err(), "lines refuse values");
    }

    #[test]
    fn args_fill_in_order_and_check_kinds() {
        let size = find("format.size").expect("mapped");
        assert_eq!(
            size.args_json(&["14"]).expect("one value").as_deref(),
            Some(r#"{"FontHeight.Height":{"type":"float","value":"14"}}"#)
        );
        assert!(size.args_json(&[]).is_err(), "a missing value is refused");
        assert!(
            size.args_json(&["junk"]).is_err(),
            "a non-number is refused"
        );
        assert!(size.args_json(&["0"]).is_err(), "out of range is refused");
        assert!(
            size.args_json(&["18.01"]).is_err(),
            "finer than 0.05 is refused"
        );
        assert!(size.args_json(&["18.05"]).is_ok(), "0.05 steps pass");
        for bad in ["18.05000001", "18.051", "1e1", "-5", "18.", ".5", "18.5x"] {
            assert!(size.args_json(&[bad]).is_err(), "{bad} is refused");
        }
        assert!(size.args_json(&["999"]).is_ok() && size.args_json(&["1.5"]).is_ok());
        assert!(
            find("format.font")
                .expect("mapped")
                .args_json(&[" "])
                .is_err()
        );
        let bold = find("format.bold").expect("mapped");
        assert_eq!(bold.args_json(&[]).expect("none"), None);
        assert!(find("format.dialog").is_err());
    }

    #[test]
    fn kernel_lists_read_by_family() {
        let fonts = r#"{"commandName":".uno:CharFontName","commandValues":{"Arial":["10"],"Noto Sans":[]}}"#;
        assert_eq!(super::fonts(fonts).expect("fonts"), ["Arial", "Noto Sans"]);
        let styles = r#"{"commandValues":{"CharacterStyles":["Emphasis"],"ParagraphStyles":["Title","Heading 1"]}}"#;
        assert_eq!(styles_of(styles, "2"), ["Title", "Heading 1"]);
        assert_eq!(styles_of(styles, "1"), ["Emphasis"]);
        assert!(super::styles(styles, "3").is_err(), "no third family");
        assert!(super::styles("{}", "2").is_err(), "a missing family fails");
        assert!(super::fonts("not json").is_err());
        let both = r#"{"western":"Liberation Mono","asian":""}"#;
        let fonts = super::char_fonts(both).expect("fonts");
        assert_eq!(fonts, Some(["Liberation Mono".into(), String::new()]));
        assert_eq!(super::char_fonts(r#"{"unread":true}"#).expect("unread"), None);
        assert!(super::char_fonts(r#"{"western":"Arial"}"#).is_err(), "no Asian font");
    }

    fn styles_of(json: &str, family: &str) -> Vec<String> {
        super::styles(json, family).expect("styles")
    }
}
