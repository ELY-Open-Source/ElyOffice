//! ElyShow's scoreboard reads open books, saved or not.

use std::{io, time::Duration};

use ely_lok::{Doc, Worker, sheet};
use ely_office::prelude::*;
use ely_ui::Ask;
use gpui::{App, Global};

use crate::book::Book;

/// More than this no longer reads from a seat.
const MOST_ROWS: usize = 20;
const MOST_COLUMNS: usize = 8;

const GONE: &str = "that workbook is no longer open in ElyTable";

/// Longest an unchanged read waits; asks expire at 90 s.
const HOLD: Duration = Duration::from_secs(60);

/// Reads waiting for their book to change.
#[derive(Default)]
struct Held {
    waits: Vec<Wait>,
    serial: u64,
}

impl Global for Held {}

struct Wait {
    serial: u64,
    ask: Ask,
    key: String,
    sheet: String,
    seen: String,
}

/// An open tab, held so it stays while read.
struct Open {
    /// This process and the tab: a restart's differ.
    key: String,
    name: String,
    path: Option<String>,
    doc: Doc,
    /// Moves with each edit and each redraw.
    mark: String,
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
                mark: {
                    let (revision, redraws) = tab.view.read(cx).changes();
                    format!("{revision}.{redraws}")
                },
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
        ["score.read", key, sheet, seen] => hold(ask, key, sheet.to_owned(), seen.to_owned(), cx),
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
        for Open { key, name, path, doc, worker, busy: _busy, .. } in open {
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

/// Reads once the book's mark leaves `seen`.
fn hold(ask: Ask, key: &str, sheet: String, seen: String, cx: &mut App) {
    match open_books(cx).map(|open| open.into_iter().find(|open| open.key == key)) {
        Ok(Some(open)) if open.mark == seen => {}
        Ok(_) => return read(ask, key, sheet, cx),
        Err(error) => return ask.answer(Err(error.to_string())),
    }
    let held = cx.default_global::<Held>();
    held.serial += 1;
    let serial = held.serial;
    held.waits.push(Wait { serial, ask, key: key.to_owned(), sheet, seen });
    cx.spawn(async move |cx| {
        cx.background_executor().timer(HOLD).await;
        cx.update(|cx| {
            let waits = &mut cx.default_global::<Held>().waits;
            if let Some(at) = waits.iter().position(|wait| wait.serial == serial) {
                let wait = waits.remove(at);
                wait.ask.answer(Ok(serde_json::json!({ "mark": wait.seen }).to_string()));
            }
        });
    })
    .detach();
}

/// Reads for the waits whose book moved or closed.
pub(crate) fn changed(cx: &mut App) {
    if cx.try_global::<Held>().is_none_or(|held| held.waits.is_empty()) {
        return;
    }
    let waits = std::mem::take(&mut cx.default_global::<Held>().waits);
    let marks: Vec<(String, String)> = match open_books(cx) {
        Ok(open) => open.into_iter().map(|open| (open.key, open.mark)).collect(),
        Err(error) => return waits.into_iter().for_each(|wait| wait.ask.answer(Err(error.to_string()))),
    };
    for wait in waits {
        if marks.iter().any(|(key, mark)| *key == wait.key && *mark == wait.seen) {
            cx.default_global::<Held>().waits.push(wait);
        } else {
            read(wait.ask, &wait.key, wait.sheet, cx);
        }
    }
}

/// Answers the book's mark and the sheet's table.
fn read(ask: Ask, key: &str, name: String, cx: &mut App) {
    let found = open_books(cx).map(|open| open.into_iter().find(|open| open.key == key));
    let Open { doc, mark, worker, busy, .. } = match found {
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
        let answer = read.await.map(|rows| serde_json::json!({ "mark": mark, "rows": rows }).to_string());
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
