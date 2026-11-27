//! Live scoreboards: ElyTable sheets drawn over the show.

use std::{collections::HashMap, io, time::Duration};

use ely_platform::{AppId, instance};
use gpui::{App, Context, SharedString, Task};

use super::Show;
use crate::deck::Deck;

pub(crate) mod dialog;
mod clear;
mod draw;
mod editor;

pub use draw::{Band, overlay};

/// How long a failed read waits to try again.
const RETRY: Duration = Duration::from_secs(1);

/// A sheet of a workbook open in ElyTable.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Source {
    /// ElyTable's key for the open book.
    pub book: String,
    /// The book's name, as its tab shows.
    pub name: String,
    pub sheet: String,
}

/// Sends words to ElyTable; its answer.
pub type Ask = fn(&[&str]) -> io::Result<String>;

pub fn ask_table(words: &[&str]) -> io::Result<String> {
    instance::ask(AppId::Table, &serde_json::json!(words).to_string())
}

fn invalid(why: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why)
}

fn parsed(json: &str) -> io::Result<serde_json::Value> {
    serde_json::from_str(json).map_err(|error| invalid(format!("ElyTable's answer: {error}")))
}

/// Every sheet of every book open in ElyTable.
pub fn sheets(ask: Ask) -> io::Result<Vec<Source>> {
    let files = parsed(&ask(&["docs"])?)?;
    let list = || -> io::Result<Vec<serde_json::Value>> {
        let value = parsed(&ask(&["score.list"])?)?;
        value.as_array().cloned().ok_or_else(|| invalid(format!("ElyTable listed no books: {value}")))
    };
    let mut books = list()?;
    // Files behind other tabs load once asked for.
    let waiting: Vec<&str> = (files.as_array().into_iter().flatten())
        .filter_map(serde_json::Value::as_str)
        .filter(|file| !books.iter().any(|book| book["path"] == *file))
        .collect();
    if !waiting.is_empty() {
        for file in waiting {
            ask(&["score.load", file])?;
        }
        books = list()?;
    }
    let mut found = Vec::new();
    for book in &books {
        let bad = || invalid(format!("ElyTable's book is unreadable: {book}"));
        let key = book["book"].as_str().ok_or_else(bad)?;
        let name = book["name"].as_str().ok_or_else(bad)?;
        for sheet in book["sheets"].as_array().ok_or_else(bad)? {
            let sheet = sheet.as_str().ok_or_else(bad)?;
            found.push(Source { book: key.into(), name: name.into(), sheet: sheet.into() });
        }
    }
    Ok(found)
}

/// The sheet's table; empty when its A1 is.
pub fn rows(source: &Source, ask: Ask) -> io::Result<Vec<Vec<String>>> {
    let read = changed(source, None, ask)?;
    read.rows.ok_or_else(|| invalid("ElyTable gave no rows".into()))
}

/// What ElyTable gave: its book's mark, and rows if read.
pub struct Read {
    pub mark: String,
    pub rows: Option<Vec<Vec<String>>>,
}

/// The table once the book leaves `seen`; ElyTable waits.
pub fn changed(source: &Source, seen: Option<&str>, ask: Ask) -> io::Result<Read> {
    let mut words = vec!["score.read", source.book.as_str(), source.sheet.as_str()];
    words.extend(seen);
    let value = parsed(&ask(&words)?)?;
    let bad = || invalid(format!("ElyTable's rows are unreadable: {value}"));
    let mark = value["mark"].as_str().ok_or_else(bad)?.to_owned();
    let Some(rows) = value.get("rows") else {
        return Ok(Read { mark, rows: None });
    };
    let cells = |row: &serde_json::Value| -> io::Result<Vec<String>> {
        let row = row.as_array().ok_or_else(bad)?;
        row.iter().map(|cell| cell.as_str().map(str::to_owned).ok_or_else(bad)).collect()
    };
    let rows = rows.as_array().ok_or_else(bad)?.iter().map(cells).collect::<io::Result<Vec<_>>>()?;
    match rows.iter().all(|row| row.len() == rows[0].len()) {
        true => Ok(Read { mark, rows: Some(rows) }),
        false => Err(invalid("ElyTable's rows differ in length".into())),
    }
}

/// `changed` on its own thread; ElyTable may hold it.
async fn held(source: Source, seen: Option<String>, ask: Ask) -> io::Result<Read> {
    let (done, read) = futures::channel::oneshot::channel();
    std::thread::Builder::new()
        .name("scoreboard-read".into())
        .spawn(move || done.send(changed(&source, seen.as_deref(), ask)))?;
    read.await.map_err(|_| io::Error::other("the scoreboard read ended unheard"))?
}

/// Where the scoreboard shows.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Look {
    #[default]
    Center,
    Corner,
    Full,
    Off,
}

impl Look {
    pub fn next(self) -> Look {
        match self {
            Look::Center => Look::Corner,
            Look::Corner => Look::Full,
            Look::Full => Look::Off,
            Look::Off => Look::Center,
        }
    }
}

/// What one sheet last gave.
#[derive(Default)]
pub struct Scores {
    pub rows: Vec<Vec<String>>,
    /// Why the last read failed; old scores stay.
    pub failed: Option<SharedString>,
}

/// The scoreboards a show or the editor follows.
pub struct Board {
    /// Each slide's sheet, by place in the deck.
    slides: Vec<Option<Source>>,
    read: HashMap<Source, Scores>,
    pub look: Look,
    _polls: Vec<Task<()>>,
}

/// Where its owner keeps a board.
type Slot<T> = fn(&mut T) -> Option<&mut Board>;

impl Board {
    /// Reads the slides' sheets while held; None for none.
    pub fn follow<T: 'static>(slides: Vec<Option<Source>>, ask: Ask, slot: Slot<T>, cx: &mut Context<T>) -> Option<Board> {
        let mut sources: Vec<Source> = Vec::new();
        for source in slides.iter().flatten() {
            if !sources.contains(source) {
                sources.push(source.clone());
            }
        }
        if sources.is_empty() {
            return None;
        }
        log::info!("show: follows {} scoreboards", sources.len());
        // Each sheet waits on its own; ElyTable answers on change.
        let polls = (sources.into_iter())
            .map(|source| {
                cx.spawn(async move |this, cx| {
                    let mut seen: Option<String> = None;
                    loop {
                        let read = held(source.clone(), seen.clone(), ask).await;
                        let failed = read.is_err();
                        let rows = match read {
                            Ok(Read { mark, rows: Some(rows) }) => Ok((mark, rows)),
                            Ok(Read { mark, rows: None }) => {
                                seen = Some(mark);
                                continue;
                            }
                            Err(error) => Err(error),
                        };
                        seen = rows.as_ref().ok().map(|(mark, _)| mark.clone());
                        let taken = this.update(cx, |owner, cx| {
                            let differs = slot(owner).is_some_and(|board| board.take(&source, rows.map(|(_, rows)| rows), cx));
                            if differs {
                                cx.notify();
                            }
                        });
                        if taken.is_err() {
                            return;
                        }
                        if failed {
                            cx.background_executor().timer(RETRY).await;
                        }
                    }
                })
            })
            .collect();
        Some(Board { slides, read: HashMap::new(), look: Look::default(), _polls: polls })
    }

    /// Keeps what a read gave; true when it differs.
    fn take(&mut self, source: &Source, read: io::Result<Vec<Vec<String>>>, cx: &App) -> bool {
        let known = self.read.contains_key(source);
        let scores = self.read.entry(source.clone()).or_default();
        match read {
            Ok(rows) => {
                let changed = !known || scores.rows != rows || scores.failed.is_some();
                if changed {
                    *scores = Scores { rows, failed: None };
                }
                changed
            }
            Err(error) => {
                let why = Deck::text(cx, "show.score.failed", &[("error", &error.to_string())]);
                let changed = scores.failed.as_ref() != Some(&why);
                if changed {
                    log::warn!("show: a scoreboard did not read: {error}");
                    scores.failed = Some(why);
                }
                changed
            }
        }
    }

    /// Whether slide `part` has a sheet.
    pub fn linked(&self, part: usize) -> bool {
        self.slides.get(part).is_some_and(Option::is_some)
    }

    /// The scores of slide `part`'s sheet, once read.
    pub fn of(&self, part: usize) -> Option<&Scores> {
        self.read.get(self.slides.get(part)?.as_ref()?)
    }

    /// Whether it follows exactly these slides' sheets.
    pub fn follows(&self, slides: &[Option<Source>]) -> bool {
        self.slides == slides
    }

    /// Takes over what `old` had read.
    pub fn inherit(&mut self, old: Board) {
        self.read = old.read;
    }
}

impl Show {
    /// Reads the slides' sheets while the show runs.
    pub fn follow_scores(&mut self, slides: Vec<Option<Source>>, ask: Ask, cx: &mut Context<Self>) {
        self.board = Board::follow(slides, ask, |show: &mut Show| show.board.as_mut(), cx);
    }

    pub fn cycle_scores(&mut self, cx: &mut Context<Self>) {
        if let Some(board) = &mut self.board {
            board.look = board.look.next();
            log::info!("show: the scoreboard shows as {:?}", board.look);
            cx.notify();
        }
    }

    /// What went wrong, for whoever runs the show.
    pub fn warning(&self) -> Option<SharedString> {
        let failed = || self.board.as_ref()?.of(self.part()?)?.failed.clone();
        self.notice.clone().or_else(failed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(words: &[&str]) -> io::Result<String> {
        Ok(match words {
            ["docs"] => r#"["/late.ods"]"#.into(),
            ["score.list"] => r#"[{"book":"7:0","name":"Untitled 1","path":null,"sheets":["A","B"]},{"book":"7:1","name":"late.ods","path":"/late.ods","sheets":["C"]}]"#.into(),
            ["score.read", "7:0", "A"] => r#"{"mark":"1","rows":[["Lions","3"]]}"#.into(),
            ["score.read", "7:0", "B"] => r#"{"mark":"1","rows":[["Lions","3"],["Eagles"]]}"#.into(),
            _ => return Err(io::Error::other(format!("{words:?}"))),
        })
    }

    #[test]
    fn every_books_sheets_are_offered_and_read() {
        let found = sheets(table).expect("sheets");
        let names: Vec<_> = found.iter().map(|source| (source.name.as_str(), source.sheet.as_str())).collect();
        assert_eq!(names, [("Untitled 1", "A"), ("Untitled 1", "B"), ("late.ods", "C")]);
        assert_eq!(rows(&found[0], table).expect("rows"), [["Lions", "3"]]);
        assert!(rows(&found[1], table).is_err(), "uneven rows fail");
    }

    #[test]
    fn the_key_goes_center_corner_full_off() {
        assert_eq!(Look::default().next(), Look::Corner);
        assert_eq!(Look::Corner.next(), Look::Full);
        assert_eq!(Look::Full.next(), Look::Off);
        assert_eq!(Look::Off.next(), Look::Center);
    }
}
