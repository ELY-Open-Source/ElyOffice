//! `sheet`, `rows`, `columns`, `range merge`, `format` by address (#107).

use ely_lok::sheet::structure::{self, Refused};
use ely_office::{prelude::*, remote};
use ely_ui::Ask;
use gpui::Context;

use crate::book::Book;

const SHEET_MISSING: &str = "{sheet}: no such sheet";
const SHEET_TAKEN: &str = "{name}: a sheet with this name already exists";
const SHEET_NAME: &str = "invalid sheet name \"{name}\": cannot be empty, start/end with ', or use []*?:/\\";
const SHEET_LAST: &str = "{sheet}: cannot remove the last visible sheet";
const SHEET_POSITION: &str = "position {position} is out of range (must be 1 to {last})";
const SHEET_PROTECTED: &str = "{sheet}: sheet is protected; unprotect it in ElyTable first";
const ROWS_ADDRESS: &str = "{address}: invalid row address (use e.g. Sheet1!5 or Sheet1!5:7)";
const COLUMNS_ADDRESS: &str = "{address}: invalid column address (use e.g. Sheet1!C or Sheet1!C:E)";
const LINES_EDGE: &str = "{address}: inserting would push existing content past the end of the sheet";
const SIZE: &str = "{size}: invalid size (use a number with cm, mm, in or pt, or auto)";
const SIZE_RANGE: &str = "{size}: size is out of range (must be {min} to {max})";
const FREEZE_ADDRESS: &str = "{address}: freeze requires a single cell (e.g. Sheet1!B2)";
const MERGE_SINGLE: &str = "{address}: single cell; nothing to merge";
const MERGE_OVERLAP: &str = "{address}: already contains or overlaps merged cells; unmerge first";
const UNMERGE_NONE: &str = "{address}: no merged cells in this range";

/// Twips per unit a size may be written in.
const UNITS: [(&str, f64); 4] = [("cm", 1440.0 / 2.54), ("mm", 144.0 / 2.54), ("in", 1440.0), ("pt", 20.0)];
/// Sizes from 1 pt to Calc's 1 m.
const SIZES: (u32, u32) = (20, 56693);

/// The words sent for `sheet ...`.
pub(super) fn sheet(words: &[&str]) -> Result<Vec<String>, String> {
    Ok(match *words {
        ["list", path] => vec!["sheet.list".into(), remote::absolute(path)?],
        ["add", path, name] => sent("sheet.add", path, name, "")?,
        ["add", path, name, at] => sent("sheet.add", path, name, at)?,
        ["remove", path, sheet] => sent("sheet.remove", path, sheet, "")?,
        ["rename", path, sheet, name] => sent("sheet.rename", path, sheet, name)?,
        ["freeze", path, address] => sent("sheet.freeze", path, address, "")?,
        _ => return Err("unknown command".into()),
    })
}

/// The words sent for `rows ...`.
pub(super) fn rows(words: &[&str]) -> Result<Vec<String>, String> {
    lines("rows", "height", words)
}

/// The words sent for `columns ...`.
pub(super) fn columns(words: &[&str]) -> Result<Vec<String>, String> {
    lines("columns", "width", words)
}

/// `range merge|unmerge`'s words.
pub(super) fn merge(verb: &str, path: &str, address: &str) -> Result<Vec<String>, String> {
    sent(&format!("range.{verb}"), path, address, "")
}

fn lines(noun: &str, size: &str, words: &[&str]) -> Result<Vec<String>, String> {
    match *words {
        [verb @ ("insert" | "delete"), path, address] => sent(&format!("{noun}.{verb}"), path, address, ""),
        [verb, path, address, value] if verb == size => {
            twips(value)?;
            sent(&format!("{noun}.{size}"), path, address, value)
        }
        _ => Err("unknown command".into()),
    }
}

/// The address and value as the kernel takes them.
fn for_kernel(verb: &str, address: &str, value: &str) -> (String, String) {
    let lines = verb.starts_with("rows.") || verb.starts_with("columns.");
    let line = address.rsplit_once('!').map_or(address, |(_, line)| line);
    // One row or column: its own span.
    let address = if lines && !line.contains(':') { format!("{address}:{line}") } else { address.to_owned() };
    let value = match verb {
        "rows.height" | "columns.width" => twips(value).unwrap_or_default(),
        // Not a whole number: refused as out of range.
        "sheet.add" if !value.bytes().all(|b| b.is_ascii_digit()) => "0".into(),
        _ => value.to_owned(),
    };
    (address, value)
}

fn sent(verb: &str, path: &str, address: &str, value: &str) -> Result<Vec<String>, String> {
    Ok(vec![verb.into(), remote::absolute(path)?, address.into(), value.into()])
}

/// SIZE in twips, or `auto`.
fn twips(size: &str) -> Result<String, String> {
    if size == "auto" {
        return Ok(size.into());
    }
    let bad = || SIZE.replace("{size}", size);
    let (number, per) = UNITS.iter().find_map(|(unit, per)| size.strip_suffix(unit).map(|number| (number, per))).ok_or_else(bad)?;
    let amount: f64 = number.parse().map_err(|_| bad())?;
    let twips = (amount * per).round();
    if !twips.is_finite() || twips < f64::from(SIZES.0) || twips > f64::from(SIZES.1) {
        return Err(SIZE_RANGE.replace("{size}", size).replace("{min}", "1pt").replace("{max}", "100cm"));
    }
    Ok(format!("{twips}"))
}

/// The writer's message for a refusal of `verb` on `address`.
fn refused(why: Refused, verb: &str, address: &str, value: &str) -> String {
    let named = |template: &str, name: &str| template.replace("{name}", name);
    let new_name = if verb == "sheet.add" { address } else { value };
    match why {
        Refused::Sheet(sheet) => SHEET_MISSING.replace("{sheet}", &sheet),
        Refused::Protected(sheet) => SHEET_PROTECTED.replace("{sheet}", &sheet),
        Refused::Name => named(SHEET_NAME, new_name),
        Refused::Taken => named(SHEET_TAKEN, new_name),
        Refused::Position(last) => SHEET_POSITION.replace("{position}", value).replace("{last}", &last.to_string()),
        Refused::Last(sheet) => SHEET_LAST.replace("{sheet}", &sheet),
        Refused::Address => {
            let template = match verb.split_once('.').map_or(verb, |(noun, _)| noun) {
                "rows" => ROWS_ADDRESS,
                "columns" => COLUMNS_ADDRESS,
                "sheet" => FREEZE_ADDRESS,
                _ => super::range::ADDRESS,
            };
            template.replace("{address}", address)
        }
        Refused::Edge => LINES_EDGE.replace("{address}", address),
        Refused::Size => SIZE_RANGE.replace("{size}", value).replace("{min}", "1pt").replace("{max}", "100cm"),
        Refused::Single => MERGE_SINGLE.replace("{address}", address),
        Refused::Merged => MERGE_OVERLAP.replace("{address}", address),
        Refused::Unmerged => UNMERGE_NONE.replace("{address}", address),
        Refused::Number => super::format::NUMBER.replace("{value}", super::format::number_code(value)),
    }
}

impl Book {
    /// Answers `sheet.*`, `rows.*`, `columns.*`, `range.merge|unmerge|format`.
    pub(super) fn answer_structure(&mut self, id: u64, words: Vec<String>, ask: Ask, cx: &mut Context<Self>) {
        match <[String; 2]>::try_from(words) {
            Ok([verb, _]) if verb == "sheet.list" => self.sheet_list(id, ask, cx),
            Ok(_) => ask.answer(Err("unknown command".into())),
            Err(words) => match <[String; 4]>::try_from(words) {
                Ok([verb, _, address, value]) => self.structure(id, verb, address, value, ask, cx),
                Err(_) => ask.answer(Err("unknown command".into())),
            },
        }
    }

    fn sheet_list(&mut self, id: u64, ask: Ask, cx: &mut Context<Self>) {
        self.read_in(
            id,
            move |worker, doc| structure::sheets(&worker.values(doc, structure::SHEETS)?),
            move |_, read, _, _| {
                let shown = |sheets: Vec<structure::Sheet>| {
                    let list: Vec<_> = sheets.iter().map(|sheet| serde_json::json!({ "name": sheet.name, "used": sheet.used, "hidden": sheet.hidden })).collect();
                    serde_json::Value::from(list).to_string()
                };
                ask.answer(read.map(shown).map_err(|error| error.to_string()));
            },
            cx,
        );
    }

    fn structure(&mut self, id: u64, verb: String, address: String, value: String, ask: Ask, cx: &mut Context<Self>) {
        let (to, given) = for_kernel(&verb, &address, &value);
        let query = structure::check_query(&verb, &to, &given);
        let command = if verb == "sheet.freeze" { "structure.freeze" } else { "structure" };
        let sent_verb = verb.clone();
        self.write_in(
            id,
            move |worker, doc| match structure::checked(&worker.values(doc, &query)?)? {
                Ok(_) => worker.run(doc, command, &[&sent_verb, &to, &given]).map(Ok),
                Err(why) => Ok(Err(why)),
            },
            move |_, done, _, _| {
                let done = done.map_err(|error| error.to_string()).and_then(|done| done.map_err(|why| refused(why, &verb, &address, &value)));
                ask.answer(done.map(|()| String::new()));
            },
            cx,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_a_number_and_unit_or_auto() {
        assert_eq!(twips("2.54cm"), Ok("1440".into()));
        assert_eq!(twips("10mm"), Ok("567".into()));
        assert_eq!(twips("1in"), Ok("1440".into()));
        assert_eq!(twips("12pt"), Ok("240".into()));
        assert_eq!(twips("auto"), Ok("auto".into()));
        let range = |size: &str| SIZE_RANGE.replace("{size}", size).replace("{min}", "1pt").replace("{max}", "100cm");
        assert_eq!(twips("0.5pt"), Err(range("0.5pt")));
        assert_eq!(twips("1pt"), Ok("20".into()), "the least");
        assert_eq!(twips("100cm"), Ok("56693".into()), "Calc's 1 m, rounded as it rounds");
        assert_eq!(twips("100.01cm"), Err(range("100.01cm")));
        assert_eq!(twips("101cm"), Err(range("101cm")));
        for bad in ["2.5", "cm", "2,5cm", "2.5 cm", "-1cm", "NaNcm", "2.5CM"] {
            let said = if bad == "-1cm" || bad == "NaNcm" { range(bad) } else { SIZE.replace("{size}", bad) };
            assert_eq!(twips(bad), Err(said), "{bad}");
        }
    }

    #[test]
    fn the_kernel_gets_whole_spans_and_twips_the_user_their_words() {
        assert_eq!(for_kernel("rows.insert", "S!5", ""), ("S!5:5".into(), String::new()));
        assert_eq!(for_kernel("columns.delete", "'a!b'!C", ""), ("'a!b'!C:C".into(), String::new()));
        assert_eq!(for_kernel("rows.height", "S!1:3", "1in"), ("S!1:3".into(), "1440".into()));
        assert_eq!(for_kernel("sheet.add", "New", "2x"), ("New".into(), "0".into()));
        assert_eq!(for_kernel("sheet.add", "New", ""), ("New".into(), String::new()));
        assert_eq!(for_kernel("range.merge", "S!A1:B2", ""), ("S!A1:B2".into(), String::new()));
        let sent = rows(&["height", "a.xlsx", "S!1", "2cm"]).expect("a request");
        assert_eq!((sent[0].as_str(), sent[2].as_str(), sent[3].as_str()), ("rows.height", "S!1", "2cm"));
        assert!(std::path::Path::new(&sent[1]).is_absolute(), "{sent:?}");
        assert_eq!(columns(&["width", "a.xlsx", "S!C", "wide"]), Err(SIZE.replace("{size}", "wide")));
        assert_eq!(sheet(&["list", "a.xlsx"]).expect("a list").len(), 2);
        assert!(sheet(&["move", "a.xlsx", "S"]).is_err(), "no such verb");
    }

    #[test]
    fn refusals_read_as_the_writer_wrote_them() {
        assert_eq!(refused(Refused::Address, "rows.insert", "S!x", ""), "S!x: invalid row address (use e.g. Sheet1!5 or Sheet1!5:7)");
        assert_eq!(refused(Refused::Address, "sheet.freeze", "S!A1:B2", ""), "S!A1:B2: freeze requires a single cell (e.g. Sheet1!B2)");
        assert_eq!(refused(Refused::Position(4), "sheet.add", "New", "9"), "position 9 is out of range (must be 1 to 4)");
        assert_eq!(refused(Refused::Taken, "sheet.rename", "Old", "S"), "S: a sheet with this name already exists");
        assert_eq!(refused(Refused::Name, "sheet.add", "a/b", ""), "invalid sheet name \"a/b\": cannot be empty, start/end with ', or use []*?:/\\");
        assert_eq!(refused(Refused::Merged, "range.merge", "S!A1:B2", ""), "S!A1:B2: already contains or overlaps merged cells; unmerge first");
    }
}
