//! ElyShow's scoreboard reads open books, saved or not.

use std::io;

use ely_lok::{Doc, Worker, sheet};
use ely_office::prelude::*;
use ely_ui::Ask;
use gpui::App;

use crate::book::Book;

/// More than this no longer reads from a seat.
const MOST_ROWS: usize = 20;
const MOST_COLUMNS: usize = 8;

const GONE: &str = "that workbook is no longer open in ElyTable";

/// An open tab, held so it stays while read.
struct Open {
    /// This process and the tab: a restart's differ.
    key: String,
    name: String,
    path: Option<String>,
    doc: Doc,
    worker: std::sync::Arc<std::sync::Mutex<Worker>>,
    busy: std::rc::Rc<()>,
}

/// Every window's loaded tabs, oldest window first.
fn open_books(cx: &App) -> io::Result<Vec<Open>> {
    let hosts = ely_office::shared::Handle::<Book>::joined(cx)?.hosts();
    let mut open = Vec::new();
    for host in &hosts {
        let book = host.read(cx);
        for tab in &book.core.tabs {
            open.push(Open {
                key: format!("{}:{}", std::process::id(), tab.id),
                name: tab.name.to_string(),
                path: tab.session.path().map(|path| std::path::Path::new(path).display().to_string()),
                doc: book.read_at(tab.id, cx).unwrap_or_else(|| tab.doc.clone()),
                worker: book.core.worker.clone(),
                busy: tab.busy.clone(),
            });
        }
    }
    Ok(open)
}

/// Takes the scoreboard's asks; others come back.
pub(crate) fn route(ask: Ask, cx: &mut App) -> Option<Ask> {
    let words: Vec<String> = serde_json::from_str(&ask.request).unwrap_or_default();
    match words.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["score.list"] => list(ask, cx),
        ["score.read", key, sheet] => read(ask, key, sheet.to_owned(), cx),
        _ => return Some(ask),
    }
    None
}

/// Answers each book with its shown sheets' names.
fn list(ask: Ask, cx: &mut App) {
    let open = match open_books(cx) {
        Ok(open) => open,
        Err(error) => return ask.answer(Err(error.to_string())),
    };
    cx.spawn(async move |cx| {
        let mut books = Vec::new();
        for Open { key, name, path, doc, worker, busy: _busy } in open {
            let read = cx.background_executor().spawn(async move { ely_lok::behind(&worker, move |worker| doc.sheets(worker)).await });
            match read.await {
                Ok(sheets) => {
                    let shown: Vec<_> = sheets.list.into_iter().filter(|sheet| sheet.visible).map(|sheet| sheet.name).collect();
                    books.push(serde_json::json!({ "book": key, "name": name, "path": path, "sheets": shown }));
                }
                Err(error) => return ask.answer(Err(format!("{name}: {error}"))),
            }
        }
        ask.answer(Ok(serde_json::Value::from(books).to_string()));
    })
    .detach();
}

/// Answers the table at the sheet's A1.
fn read(ask: Ask, key: &str, name: String, cx: &mut App) {
    let found = open_books(cx).map(|open| open.into_iter().find(|open| open.key == key));
    let Open { doc, worker, busy, .. } = match found {
        Ok(Some(open)) => open,
        Ok(None) => return ask.answer(Err(GONE.into())),
        Err(error) => return ask.answer(Err(error.to_string())),
    };
    let read = cx.background_executor().spawn(async move {
        ely_lok::behind(&worker, move |worker| {
            table(|column, row| {
                let query = sheet::cell_at_query(&address(&name, column, row));
                Ok(sheet::cell_at(&worker.values(&doc, &query)?)?.text)
            })
        })
        .await
    });
    cx.spawn(async move |_| {
        let _busy = busy;
        let answer = read.await.map(|rows| serde_json::json!(rows).to_string());
        ask.answer(answer.map_err(|error| error.to_string()));
    })
    .detach();
}

/// A cell's address; columns and rows count from zero.
fn address(sheet: &str, column: usize, row: usize) -> String {
    let letter = char::from(b'A' + column as u8);
    format!("'{}'!{letter}{}", sheet.replace('\'', "''"), row + 1)
}

/// A1 starts it: row 1 gives columns, column A rows.
fn table(mut cell: impl FnMut(usize, usize) -> io::Result<String>) -> io::Result<Vec<Vec<String>>> {
    let mut shown = |column, row| cell(column, row).map(|text| text.trim().to_owned());
    let too_big = || {
        let why = format!("the scoreboard takes at most {MOST_ROWS} rows and {MOST_COLUMNS} columns");
        io::Error::new(io::ErrorKind::InvalidData, why)
    };
    let mut first = Vec::new();
    loop {
        let text = shown(first.len(), 0)?;
        if text.is_empty() {
            break;
        }
        if first.len() == MOST_COLUMNS {
            return Err(too_big());
        }
        first.push(text);
    }
    if first.is_empty() {
        return Ok(Vec::new());
    }
    let width = first.len();
    let mut rows = vec![first];
    loop {
        let name = shown(0, rows.len())?;
        if name.is_empty() {
            return Ok(rows);
        }
        if rows.len() == MOST_ROWS {
            return Err(too_big());
        }
        let mut row = vec![name];
        for column in 1..width {
            row.push(shown(column, rows.len())?);
        }
        rows.push(row);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_starts_at_a1_and_ends_at_the_first_unnamed_row() {
        let cells = |column: usize, row: usize| {
            Ok(match (column, row) {
                (0, 0) => "Team",
                (1, 0) => "Points",
                (0, 1) => "Lions",
                (1, 1) => " 3 ",
                (0, 2) => "Eagles",
                (0, 4) => "Past the gap",
                _ => "",
            }
            .to_owned())
        };
        assert_eq!(table(cells).expect("rows"), [["Team", "Points"], ["Lions", "3"], ["Eagles", ""]]);
        assert!(table(|_, _| Ok("x".into())).is_err(), "too big is refused, not cut");
        assert!(table(|_, _| Ok(String::new())).expect("blank").is_empty());
        assert_eq!(address("It's", 1, 2), "'It''s'!B3");
    }
}
