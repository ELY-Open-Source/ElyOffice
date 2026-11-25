//! `range get|set`: a block of cells by address (#107).

use ely_lok::sheet::range::{self, MOST, Refused};
use ely_office::{
    prelude::*,
    remote::{
        self,
        grid::{EMPTY, grouped, rows},
    },
};
use ely_ui::Ask;
use gpui::Context;

use crate::book::Book;

/// The words sent for `range ...`; DATA is checked here.
pub(super) fn request(words: &[&str]) -> Result<Vec<String>, String> {
    Ok(match *words {
        ["get", path, address] => vec!["range.get".into(), remote::absolute(path)?, address.into()],
        ["set", path, address, data] => {
            let rows = rows(data).map_err(|reason| DATA.replace("{reason}", &reason))?;
            let cells = rows.len() as u64 * rows[0].len() as u64;
            if cells > MOST {
                return Err(big(address, cells));
            }
            vec!["range.set".into(), remote::absolute(path)?, address.into(), range::rows_json(&rows)]
        }
        [verb @ ("merge" | "unmerge"), path, address] => super::structure::merge(verb, path, address)?,
        _ => return Err("unknown command".into()),
    })
}

const DATA: &str = "range data must be a JSON array of string rows or TSV: {reason}";
pub(super) const ADDRESS: &str = "{address}: names no range on an existing sheet";

fn big(address: &str, cells: u64) -> String {
    format!("{address}: {} cells exceeds the 1,000,000-cell limit", grouped(cells))
}

fn sized(template: &str, address: &str, rows: usize, columns: usize) -> String {
    template.replace("{address}", address).replace("{rows}", &rows.to_string()).replace("{columns}", &columns.to_string())
}

/// Why the kernel's refusal stops this command.
fn refused(why: Refused, address: &str, rows: usize, columns: usize) -> String {
    match why {
        Refused::Address => ADDRESS.replace("{address}", address),
        Refused::Size(cells) => big(address, cells),
        Refused::Shape => sized("{address}: range does not match data size ({rows} rows, {columns} columns)", address, rows, columns),
        Refused::Edge => sized("{address}: data runs past the end of the sheet ({rows} rows, {columns} columns)", address, rows, columns),
    }
}

impl Book {
    /// Answers `range.get` and `range.set`; the cursor stays.
    pub(super) fn answer_range(&mut self, id: u64, words: Vec<String>, ask: Ask, cx: &mut Context<Self>) {
        match <[String; 3]>::try_from(words) {
            Ok([verb, _, address]) if verb == "range.get" => self.range_get(id, address, ask, cx),
            Ok(_) => ask.answer(Err("unknown command".into())),
            Err(words) => match <[String; 4]>::try_from(words) {
                Ok([verb, _, address, rows]) if verb == "range.set" => self.range_set(id, address, rows, ask, cx),
                _ => ask.answer(Err("unknown command".into())),
            },
        }
    }

    fn range_get(&mut self, id: u64, address: String, ask: Ask, cx: &mut Context<Self>) {
        let query = range::read_query(&address);
        self.read_in(
            id,
            move |worker, doc| range::read(&worker.values(doc, &query)?),
            move |_, read, _, _| {
                let shown = read.map_err(|error| error.to_string()).and_then(|read| read.map_err(|why| refused(why, &address, 0, 0)));
                ask.answer(shown.map(|range| {
                    let rows: Vec<Vec<_>> = range.rows.iter().map(|row| row.iter().map(|cell| serde_json::json!({ "text": cell.text, "input": cell.input })).collect()).collect();
                    serde_json::json!({ "address": range.address, "rows": rows }).to_string()
                }));
            },
            cx,
        );
    }

    fn range_set(&mut self, id: u64, address: String, rows: String, ask: Ask, cx: &mut Context<Self>) {
        let data: Vec<Vec<String>> = match serde_json::from_str(&rows) {
            Ok(data) => data,
            Err(error) => return ask.answer(Err(DATA.replace("{reason}", &error.to_string()))),
        };
        let (height, width) = (data.len(), data.first().map_or(0, Vec::len));
        if width == 0 {
            return ask.answer(Err(DATA.replace("{reason}", EMPTY)));
        }
        let query = range::target_query(&address, height, width);
        self.write_in(
            id,
            move |worker, doc| match range::target(&worker.values(doc, &query)?)? {
                Ok(to) => worker.run(doc, "range.set", &[&to, &rows]).map(Ok),
                Err(why) => Ok(Err(why)),
            },
            move |_, done, _, _| {
                let done = done.map_err(|error| error.to_string()).and_then(|done| done.map_err(|why| refused(why, &address, height, width)));
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
    fn the_words_carry_the_file_and_json_rows() {
        let sent = request(&["set", "a.xlsx", "S!B2", "x\ty"]).expect("a request");
        assert_eq!((sent[0].as_str(), sent[2].as_str(), sent[3].as_str()), ("range.set", "S!B2", r#"[["x","y"]]"#));
        assert!(std::path::Path::new(&sent[1]).is_absolute(), "{sent:?}");
        assert_eq!(request(&["get", "a.xlsx", "S"]).expect("a read")[0], "range.get");
        assert!(request(&["get", "a.xlsx"]).is_err(), "too few words");
        let wide = "x\t".repeat(1001) + "x";
        let tall = std::iter::repeat_n(wide.as_str(), 1000).collect::<Vec<_>>().join("\n");
        assert_eq!(request(&["set", "a.xlsx", "S!A1", &tall]), Err("S!A1: 1,002,000 cells exceeds the 1,000,000-cell limit".into()));
    }
}
