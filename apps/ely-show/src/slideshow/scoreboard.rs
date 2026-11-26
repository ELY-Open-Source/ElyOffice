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

/// How often the sheets are read again.
const EVERY: Duration = Duration::from_millis(700);

/// A sheet of a workbook open in ElyTable.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Source {
    /// ElyTable's key for the open book; empty once reloaded.
    pub book: String,
    /// The book's file, once it has one.
    pub path: Option<String>,
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
        let path = match &book["path"] {
            serde_json::Value::Null => None,
            path => Some(path.as_str().ok_or_else(bad)?.to_owned()),
        };
        for sheet in book["sheets"].as_array().ok_or_else(bad)? {
            let sheet = sheet.as_str().ok_or_else(bad)?;
            found.push(Source { book: key.into(), path: path.clone(), name: name.into(), sheet: sheet.into() });
        }
    }
    Ok(found)
}

impl Source {
    /// Whether `open` is this kept sheet: by file, else name.
    pub fn is(&self, open: &Source) -> bool {
        let book = match &self.path {
            Some(path) => open.path.as_ref() == Some(path),
            None => self.name == open.name,
        };
        self.sheet == open.sheet && (book || (!self.book.is_empty() && self.book == open.book))
    }
}

/// A kept sheet that ElyTable does not hold just once.
#[derive(Debug)]
pub struct Unfound {
    name: String,
    sheet: String,
    several: bool,
}

impl std::fmt::Display for Unfound {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let how = if self.several { "open several times" } else { "not open" };
        write!(f, "{} / {} is {how}", self.name, self.sheet)
    }
}

impl std::error::Error for Unfound {}

/// ElyTable's key for a kept sheet, among the open ones.
fn find(source: &Source, ask: Ask) -> io::Result<String> {
    let unfound = |several| {
        let (name, sheet) = (source.name.clone(), source.sheet.clone());
        io::Error::other(Unfound { name, sheet, several })
    };
    let open = sheets(ask).map_err(|error| match error.kind() {
        // No ElyTable answers: the book is not open.
        io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound => unfound(false),
        _ => error,
    })?;
    let mut found = open.into_iter().filter(|open| source.is(open));
    match (found.next(), found.next()) {
        (Some(open), None) => Ok(open.book),
        (first, _) => Err(unfound(first.is_some())),
    }
}

/// Kept scores take their book's present name and file.
pub fn renamed(mut held: Vec<crate::keep::Held>, ask: Ask) -> Vec<crate::keep::Held> {
    let linked = held.iter().filter_map(|held| held.score.as_ref()).any(|score| !score.book.is_empty());
    if !linked {
        return held;
    }
    let open = match sheets(ask) {
        Ok(open) => open,
        Err(error) => {
            log::warn!("show: scoreboards keep their linked names: {error}");
            return held;
        }
    };
    for score in held.iter_mut().filter_map(|held| held.score.as_mut()) {
        let now = open.iter().find(|open| open.book == score.book && open.sheet == score.sheet);
        if let Some(now) = now {
            *score = now.clone();
        }
    }
    held
}

/// The sheet's table; empty when its A1 is.
pub fn rows(source: &Source, ask: Ask) -> io::Result<Vec<Vec<String>>> {
    if source.book.is_empty() {
        return read(&find(source, ask)?, &source.sheet, ask);
    }
    match read(&source.book, &source.sheet, ask) {
        Ok(rows) => Ok(rows),
        // A restarted ElyTable has new keys.
        Err(first) => match find(source, ask) {
            Ok(book) if book != source.book => read(&book, &source.sheet, ask),
            Err(lost) if lost.get_ref().is_some_and(|inner| inner.is::<Unfound>()) => Err(lost),
            _ => Err(first),
        },
    }
}

fn read(book: &str, sheet: &str, ask: Ask) -> io::Result<Vec<Vec<String>>> {
    let value = parsed(&ask(&["score.read", book, sheet])?)?;
    let bad = || invalid(format!("ElyTable's rows are unreadable: {value}"));
    let cells = |row: &serde_json::Value| -> io::Result<Vec<String>> {
        let row = row.as_array().ok_or_else(bad)?;
        row.iter().map(|cell| cell.as_str().map(str::to_owned).ok_or_else(bad)).collect()
    };
    let rows = value.as_array().ok_or_else(bad)?.iter().map(cells).collect::<io::Result<Vec<_>>>()?;
    match rows.iter().all(|row| row.len() == rows[0].len()) {
        true => Ok(rows),
        false => Err(invalid("ElyTable's rows differ in length".into())),
    }
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
    _poll: Task<()>,
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
        // All are kept fresh: a new slide shows at once.
        let poll = cx.spawn(async move |this, cx| {
            loop {
                for source in &sources {
                    let from = source.clone();
                    let read = cx.background_executor().spawn(async move { rows(&from, ask) }).await;
                    let taken = this.update(cx, |owner, cx| {
                        let changed = slot(owner).is_some_and(|board| board.take(source, read, cx));
                        if changed {
                            cx.notify();
                        }
                    });
                    if taken.is_err() {
                        return;
                    }
                }
                cx.background_executor().timer(EVERY).await;
            }
        });
        Some(Board { slides, read: HashMap::new(), look: Look::default(), _poll: poll })
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
                let why = match error.get_ref().and_then(|inner| inner.downcast_ref::<Unfound>()) {
                    Some(lost) => {
                        let key = if lost.several { "show.score.several" } else { "show.score.closed" };
                        Deck::text(cx, key, &[("name", &lost.name), ("sheet", &lost.sheet)])
                    }
                    None => Deck::text(cx, "show.score.failed", &[("error", &error.to_string())]),
                };
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
            ["score.read", "7:0", "A"] => r#"[["Lions","3"]]"#.into(),
            ["score.read", "7:0", "B"] => r#"[["Lions","3"],["Eagles"]]"#.into(),
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
