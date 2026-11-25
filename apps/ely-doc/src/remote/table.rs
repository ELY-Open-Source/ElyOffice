//! `table insert FILE DATA [--after FIND] [--header]` (#107).

use ely_office::remote::{self, grid};

use super::text::FIND_PARAGRAPH;

const DATA: &str = "table data must be a JSON array of string rows or TSV: {reason}";
const BIG: &str = "table of {cells} cells exceeds the 1,000,000-cell limit";
const CONTROL: &str = "row {row}, column {column} contains reserved U+001E or U+001F";

/// Cells one table takes at most.
const MOST: u64 = 1_000_000;

/// The kernel's row and cell separators.
const ROW: char = '\u{1e}';
const CELL: char = '\u{1f}';

/// The words sent: verb, file, cells, FIND or empty, header.
pub(super) fn request(words: &[&str]) -> Result<Vec<String>, String> {
    let ["insert", path, data, flags @ ..] = words else {
        return Err("unknown command".into());
    };
    let (mut after, mut header) = (None, false);
    let mut flags = flags.iter();
    while let Some(flag) = flags.next() {
        match *flag {
            "--header" if !header => header = true,
            "--after" if after.is_none() => after = Some(*flags.next().ok_or("unknown command")?),
            _ => return Err("unknown command".into()),
        }
    }
    if after.is_some_and(|find| find.is_empty() || find.contains(['\n', '\r'])) {
        return Err(FIND_PARAGRAPH.into());
    }
    let cells = cells(data)?;
    let header = if header { "true" } else { "false" };
    Ok(vec!["table.insert".into(), remote::absolute(path)?, cells, after.unwrap_or_default().into(), header.into()])
}

/// DATA as the kernel's text; cell line ends break lines.
fn cells(data: &str) -> Result<String, String> {
    let wrong = |reason: String| DATA.replace("{reason}", &reason);
    let rows = grid::rows(data).map_err(wrong)?;
    let count = rows.len() as u64 * rows[0].len() as u64;
    if count > MOST {
        return Err(BIG.replace("{cells}", &grid::grouped(count)));
    }
    let mut out = Vec::with_capacity(rows.len());
    for (r, row) in rows.iter().enumerate() {
        if let Some(c) = row.iter().position(|cell| cell.contains([ROW, CELL])) {
            return Err(wrong(CONTROL.replace("{row}", &(r + 1).to_string()).replace("{column}", &(c + 1).to_string())));
        }
        let row: Vec<String> = row.iter().map(|cell| cell.replace("\r\n", "\n").replace('\r', "\n")).collect();
        out.push(row.join(&CELL.to_string()));
    }
    Ok(out.join(&ROW.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_becomes_the_kernels_rows_and_cells() {
        let sent = request(&["insert", "a.docx", "Name\tQty\nTea\t3\n", "--header"]).expect("fine");
        assert_eq!((sent[0].as_str(), sent[2].as_str()), ("table.insert", "Name\u{1f}Qty\u{1e}Tea\u{1f}3"));
        assert!(std::path::Path::new(&sent[1]).is_absolute(), "{sent:?}");
        assert_eq!((sent[3].as_str(), sent[4].as_str()), ("", "true"));
        let sent = request(&["insert", "a.docx", r#"[["a\r\nb",""]]"#, "--after", "第一章"]).expect("fine");
        assert_eq!(sent[2], "a\nb\u{1f}", "a cell's line end breaks its line");
        assert_eq!((sent[3].as_str(), sent[4].as_str()), ("第一章", "false"));
        let both = request(&["insert", "a.docx", "x", "--header", "--after", "y"]).expect("either order");
        assert_eq!((both[3].as_str(), both[4].as_str()), ("y", "true"));
        assert_eq!(request(&["insert", "a.docx", r#"[[""]]"#]).expect("an empty cell")[2], "");
        assert_eq!(request(&["insert", "a.docx", r#"[[" "]]"#, "--after", "x"]).expect("a blank cell")[2], " ");
    }

    #[test]
    fn bad_data_or_flags_are_refused() {
        let data = |reason: &str| Err(DATA.replace("{reason}", reason));
        assert_eq!(request(&["insert", "a.docx", "[]"]), data(grid::EMPTY));
        assert_eq!(request(&["insert", "a.docx", "a\tb\nc"]), data("row 2 has 1 cells; row 1 has 2"));
        let control = |row: &str, column: &str| data(&CONTROL.replace("{row}", row).replace("{column}", column));
        assert_eq!(request(&["insert", "a.docx", r#"[["ok"],["a\u001fb"]]"#]), control("2", "1"));
        assert_eq!(request(&["insert", "a.docx", "a\tb\u{1e}c"]), control("1", "2"), "raw in TSV too");
        let wide = "x\t".repeat(1001) + "x";
        let tall = std::iter::repeat_n(wide.as_str(), 1000).collect::<Vec<_>>().join("\n");
        assert_eq!(request(&["insert", "a.docx", &tall]), Err("table of 1,002,000 cells exceeds the 1,000,000-cell limit".into()));
        assert_eq!(request(&["insert", "a.docx", "x", "--after", ""]), Err(FIND_PARAGRAPH.into()));
        assert_eq!(request(&["insert", "a.docx", "x", "--after", "a\nb"]), Err(FIND_PARAGRAPH.into()));
        for flags in [&["--after"][..], &["--header", "--header"], &["--style", "x"], &["--after", "a", "--after", "b"]] {
            let words = [&["insert", "a.docx", "x"][..], flags].concat();
            assert!(request(&words).is_err(), "{flags:?}");
        }
        assert!(request(&["insert", "a.docx"]).is_err(), "no DATA");
    }
}
