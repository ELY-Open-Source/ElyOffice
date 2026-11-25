//! DATA for `range set` and `table insert`: JSON or TSV.

pub const EMPTY: &str = "no rows, or a row has no cells";
const RAGGED: &str = "row {row} has {cells} cells; row 1 has {first}";

/// DATA as rows: JSON array of string rows, else TSV.
pub fn rows(data: &str) -> Result<Vec<Vec<String>>, String> {
    let rows: Vec<Vec<String>> = if data.trim_start().starts_with('[') {
        serde_json::from_str(data).map_err(|error| error.to_string())?
    } else {
        // One line ending closes the last row; CRLF lines too.
        let body = data.strip_suffix('\n').unwrap_or(data);
        body.split('\n').map(|line| line.strip_suffix('\r').unwrap_or(line).split('\t').map(String::from).collect()).collect()
    };
    let width = rows.first().map_or(0, Vec::len);
    if width == 0 {
        return Err(EMPTY.into());
    }
    if let Some(at) = rows.iter().position(|row| row.len() != width) {
        let said = RAGGED.replace("{row}", &(at + 1).to_string()).replace("{cells}", &rows[at].len().to_string());
        return Err(said.replace("{first}", &width.to_string()));
    }
    Ok(rows)
}

/// `n` with thousands commas, as the limit reads.
pub fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_is_json_rows_or_tsv_and_a_rectangle() {
        let want = vec![vec!["a".to_string(), "=1+1".into()], vec![String::new(), "b c".into()]];
        assert_eq!(rows(r#"[["a","=1+1"],["","b c"]]"#), Ok(want.clone()));
        assert_eq!(rows("a\t=1+1\n\tb c\n"), Ok(want.clone()), "one closing line end");
        assert_eq!(rows("a\t=1+1\r\n\tb c\r\n"), Ok(want), "CRLF lines");
        assert_eq!(rows(""), Ok(vec![vec![String::new()]]), "an empty TSV is one empty cell");
        assert_eq!(rows("a\n\n"), Ok(vec![vec!["a".into()], vec![String::new()]]), "a blank line is a row");
        assert_eq!(rows("[]"), Err(EMPTY.into()));
        assert_eq!(rows("[[]]"), Err(EMPTY.into()));
        let ragged = RAGGED.replace("{row}", "2").replace("{cells}", "1").replace("{first}", "2");
        assert_eq!(rows("a\tb\nc"), Err(ragged.clone()));
        assert_eq!(rows(r#"[["a","b"],["c"]]"#), Err(ragged));
        assert!(rows(r#"[["a",1]]"#).is_err(), "cells are strings");
    }

    #[test]
    fn counts_read_with_commas() {
        assert_eq!((grouped(7), grouped(1_000), grouped(1_002_000)), ("7".into(), "1,000".into(), "1,002,000".into()));
    }
}
