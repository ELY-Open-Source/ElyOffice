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
    let books = books(ask)?;
    // Files behind other tabs load once asked for.
    let waiting: Vec<&str> = (files.as_array().into_iter().flatten())
        .filter_map(serde_json::Value::as_str)
        .filter(|file| !books.iter().any(|book| book["path"] == *file))
        .collect();
    if waiting.is_empty() {
        return sources(&books);
    }
    for file in waiting {
        ask(&["score.load", file])?;
    }
    listed(ask)
}

/// The sheets of the books ElyTable has loaded; none load.
fn listed(ask: Ask) -> io::Result<Vec<Source>> {
    sources(&books(ask)?)
}

fn books(ask: Ask) -> io::Result<Vec<serde_json::Value>> {
    let value = parsed(&ask(&["score.list"])?)?;
    value.as_array().cloned().ok_or_else(|| invalid(format!("ElyTable listed no books: {value}")))
}

fn sources(books: &[serde_json::Value]) -> io::Result<Vec<Source>> {
    let mut found = Vec::new();
    for book in books {
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
    // A keyed book is loaded; saving loads no tabs.
    let open = match listed(ask) {
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
    let read = changed(source, None, ask)?;
    read.rows.ok_or_else(|| invalid("ElyTable gave no rows".into()))
}

/// What ElyTable gave: book, mark, and rows if read.
pub struct Read {
    /// ElyTable's key for the book it read.
    pub book: String,
    pub mark: String,
    pub rows: Option<Vec<Vec<String>>>,
}

/// The table once the book leaves `seen`; ElyTable waits.
pub fn changed(source: &Source, seen: Option<&str>, ask: Ask) -> io::Result<Read> {
    if source.book.is_empty() {
        return read(&find(source, ask)?, &source.sheet, None, ask);
    }
    match read(&source.book, &source.sheet, seen, ask) {
        Ok(got) => Ok(got),
        // A restarted ElyTable has new keys; marks are per book.
        Err(first) => match find(source, ask) {
            Ok(book) if book != source.book => read(&book, &source.sheet, None, ask),
            Err(lost) if lost.get_ref().is_some_and(|inner| inner.is::<Unfound>()) => Err(lost),
            _ => Err(first),
        },
    }
}

fn read(book: &str, sheet: &str, seen: Option<&str>, ask: Ask) -> io::Result<Read> {
    let mut words = vec!["score.read", book, sheet];
    words.extend(seen);
    let value = parsed(&ask(&words)?)?;
    let bad = || invalid(format!("ElyTable's rows are unreadable: {value}"));
    let mark = value["mark"].as_str().ok_or_else(bad)?.to_owned();
    let book = book.to_owned();
    let Some(rows) = value.get("rows") else {
        return Ok(Read { book, mark, rows: None });
    };
    let cells = |row: &serde_json::Value| -> io::Result<Vec<String>> {
        let row = row.as_array().ok_or_else(bad)?;
        row.iter().map(|cell| cell.as_str().map(str::to_owned).ok_or_else(bad)).collect()
    };
    let rows = rows.as_array().ok_or_else(bad)?.iter().map(cells).collect::<io::Result<Vec<_>>>()?;
    match rows.iter().all(|row| row.len() == rows[0].len()) {
        true => Ok(Read { book, mark, rows: Some(rows) }),
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
                    // The key last read; a reopened deck has none.
                    let mut now = source.clone();
                    loop {
                        let read = held(now.clone(), seen.clone(), ask).await;
                        let failed = read.is_err();
                        if let Ok(read) = &read {
                            now.book.clone_from(&read.book);
                        }
                        let rows = match read {
                            Ok(Read { mark, rows: Some(rows), .. }) => Ok((mark, rows)),
                            Ok(Read { mark, rows: None, .. }) => {
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

    static LISTS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    /// ElyTable after a restart: `late.ods` is now 9:4.
    fn restarted(words: &[&str]) -> io::Result<String> {
        if words == ["score.list"] {
            LISTS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        Ok(match words {
            ["docs"] => r#"["/late.ods"]"#.into(),
            ["score.list"] => r#"[{"book":"9:4","name":"late.ods","path":"/late.ods","sheets":["C"]}]"#.into(),
            ["score.read", "9:4", "C"] => r#"{"mark":"5","rows":[["Owls","1"]]}"#.into(),
            ["score.read", "9:4", "C", "5"] => r#"{"mark":"5"}"#.into(),
            _ => return Err(io::Error::other(format!("{words:?}"))),
        })
    }

    #[test]
    fn a_reopened_link_finds_its_book_once_then_waits_on_it() {
        let kept = |book: &str, path: &str| Source { book: book.into(), path: Some(path.into()), name: "late.ods".into(), sheet: "C".into() };
        let read = changed(&kept("", "/late.ods"), None, restarted).expect("found by its file");
        assert_eq!((read.book.as_str(), read.rows), ("9:4", Some(vec![vec!["Owls".to_string(), "1".to_string()]])));
        let stale = changed(&kept("7:1", "/late.ods"), Some("2"), restarted).expect("an old key finds the new");
        assert_eq!((stale.book.as_str(), stale.mark.as_str()), ("9:4", "5"));
        let lists = LISTS.load(std::sync::atomic::Ordering::SeqCst);
        let waited = changed(&kept("9:4", "/late.ods"), Some("5"), restarted).expect("held");
        assert!(waited.rows.is_none(), "unchanged");
        assert_eq!(LISTS.load(std::sync::atomic::Ordering::SeqCst), lists, "a found key is not looked up again");
        match changed(&kept("", "/gone.ods"), None, restarted) {
            Ok(read) => panic!("a closed book read as {}", read.book),
            Err(lost) => assert!(lost.get_ref().is_some_and(|inner| inner.is::<Unfound>()), "{lost}"),
        }
    }

    /// ElyTable where loading a tab fails the test.
    fn unloaded(words: &[&str]) -> io::Result<String> {
        match words {
            ["score.list"] => table(words),
            _ => Err(io::Error::other(format!("saving asked {words:?}"))),
        }
    }

    #[test]
    fn saving_takes_present_names_without_loading_tabs() {
        let score = Source { book: "7:0".into(), path: None, name: "Old name".into(), sheet: "A".into() };
        let held = vec![crate::keep::Held { at: 0, score: Some(score), timer: None }];
        let renamed = renamed(held, unloaded);
        assert_eq!(renamed[0].score.as_ref().map(|score| score.name.as_str()), Some("Untitled 1"));
    }

    #[test]
    fn the_key_goes_center_corner_full_off() {
        assert_eq!(Look::default().next(), Look::Corner);
        assert_eq!(Look::Corner.next(), Look::Full);
        assert_eq!(Look::Full.next(), Look::Off);
        assert_eq!(Look::Off.next(), Look::Center);
    }
}
