//! Searching the mail held here (MAIL-09).

use rusqlite::ToSql;

use super::{Filter, MessageId, Store, Summary, search::words};
use crate::{Error, Result};

/// Where the words are looked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Field {
    #[default]
    All,
    From,
    /// To and Cc.
    To,
    Subject,
    Body,
    Files,
}

/// Words, each a word's start; quoted words go together.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Search {
    pub words: String,
    pub field: Field,
    /// From and before, seconds since the Unix epoch.
    pub dates: Option<(i64, i64)>,
    pub filter: Filter,
    /// Opened under the filter, so still listed.
    pub held: Vec<MessageId>,
}

/// Matches, newest first, and what could not be searched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub messages: Vec<Summary>,
    /// Bodies not held here, so not searched.
    pub unsearched: u64,
}

impl Store {
    /// Every account and folder; only what is held here.
    pub fn search(&self, search: &Search) -> Result<Found> {
        let matching = expression(&search.words, search.field);
        if matching.is_none() && search.dates.is_none() {
            return Err(Error::Invalid("nothing to search for".into()));
        }
        let mut clauses = vec!["NOT m.erasing".to_owned()];
        let mut values: Vec<&dyn ToSql> = Vec::new();
        if let Some(matching) = &matching {
            values.push(matching);
            clauses.push(format!(
                "m.id IN (SELECT rowid FROM search WHERE search MATCH ?{})",
                values.len()
            ));
        }
        let (since, before);
        if let Some(dates) = search.dates {
            (since, before) = dates;
            values.push(&since);
            values.push(&before);
            let at = values.len();
            clauses.push(format!("m.date >= ?{} AND m.date < ?{at}", at - 1));
        }
        let unsearched = match (search.field, &matching) {
            (Field::All | Field::Body | Field::Files, Some(_)) => self.unsearched(search.dates)?,
            _ => 0,
        };
        clauses.extend(search.filter.clause(&search.held));
        let messages = self.summaries(&clauses.join(" AND "), &values)?;
        log::info!("mail: a search found {} messages", messages.len());
        Ok(Found {
            messages,
            unsearched,
        })
    }

    /// Messages in the dates whose bodies are not here.
    fn unsearched(&self, dates: Option<(i64, i64)>) -> Result<u64> {
        let (since, before) = dates.unwrap_or((i64::MIN, i64::MAX));
        let count: i64 = self.conn.query_row(
            "SELECT count(*) FROM messages m WHERE NOT m.erasing AND m.content IS NULL
             AND (?1 IS NULL OR m.date >= ?2 AND m.date < ?3)",
            rusqlite::params![dates.map(|_| 1), since, before],
            |row| row.get(0),
        )?;
        u64::try_from(count).map_err(|_| Error::Store(format!("a negative count: {count}")))
    }
}

/// An FTS5 query; none without a word to look for.
pub(super) fn expression(text: &str, field: Field) -> Option<String> {
    let mut terms = Vec::new();
    let mut term = String::new();
    let mut quoted = false;
    let mut end = |term: &mut String, quoted: bool| {
        let spaced = words(term);
        if spaced.chars().any(char::is_alphanumeric) {
            let star = if quoted { "" } else { "*" };
            terms.push(format!("\"{}\"{star}", spaced.replace('"', "\"\"")));
        }
        term.clear();
    };
    for ch in text.chars() {
        match ch {
            '"' => {
                end(&mut term, quoted);
                quoted = !quoted;
            }
            ch if ch.is_whitespace() && !quoted => end(&mut term, false),
            ch => term.push(ch),
        }
    }
    end(&mut term, quoted);
    if terms.is_empty() {
        return None;
    }
    let all = format!("({})", terms.join(" AND "));
    Some(match field {
        Field::All => all,
        Field::From => format!("sender : {all}"),
        Field::To => format!("recipients : {all}"),
        Field::Subject => format!("subject : {all}"),
        Field::Body => format!("body : {all}"),
        Field::Files => format!("files : {all}"),
    })
}
