//! Kernel lists for references, review and checks (DOC-09 to 12).

use std::io;

use crate::Rect;

/// A comment from `.uno:ViewAnnotations`.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub id: String,
    /// "0" for a thread's first comment.
    pub parent: String,
    pub author: String,
    pub text: String,
    pub resolved: bool,
    /// ISO 8601, as the kernel gives it.
    pub at: String,
    pub anchor: Rect,
}

/// A tracked change from `.uno:AcceptTrackedChanges`.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    pub index: u64,
    pub author: String,
    pub kind: String,
    pub description: String,
    pub at: String,
    pub anchor: Option<Rect>,
}

/// Words and characters, in the document and the selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Counts {
    pub words: u64,
    pub characters: u64,
    pub selected_words: u64,
    pub selected_characters: u64,
}

/// The counts from `.uno:WordCount`.
pub fn counts(json: &str) -> io::Result<Counts> {
    let list = parse(json)?;
    let (document, selection) = (&list["document"], &list["selection"]);
    Ok(Counts {
        words: number(document, "words")?,
        characters: number(document, "characters")?,
        selected_words: number(selection, "words")?,
        selected_characters: number(selection, "characters")?,
    })
}

/// The kernel's list: the body and the caret's paragraph.
pub const READING: &str = ".uno:ElyReading";

/// The body's paragraphs; `caret` indexes the cursor's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Body {
    pub paragraphs: Vec<String>,
    pub caret: usize,
}

/// What `READING` lists; None for a cursor outside the body.
pub fn reading(json: &str) -> io::Result<Option<Body>> {
    let list = parse(json)?;
    if list["caret"].is_null() {
        return Ok(None);
    }
    let texts = entries(&list, "paragraphs")?.iter().map(|entry| entry.as_str().map(str::to_string));
    // The errors leave the document's text out.
    let paragraphs: Vec<String> = texts.collect::<Option<_>>().ok_or_else(|| invalid("a listed paragraph is no text".into()))?;
    let caret = list["caret"].as_u64().and_then(|caret| usize::try_from(caret).ok());
    match caret.filter(|caret| *caret < paragraphs.len()) {
        Some(caret) => Ok(Some(Body { paragraphs, caret })),
        None => Err(invalid(format!("the caret is in none of {} paragraphs", paragraphs.len()))),
    }
}

/// A heading from `.uno:Outline`.
#[derive(Clone, Debug, PartialEq)]
pub struct Heading {
    pub text: String,
    /// 0 for the top level.
    pub level: u64,
    pub rect: Rect,
    /// The cursor sits under it.
    pub current: bool,
}

/// The document's headings, in order.
pub fn outline(json: &str) -> io::Result<Vec<Heading>> {
    let list = parse(json)?;
    entries(&list, "outline")?
        .iter()
        .map(|entry| {
            Ok(Heading {
                text: text(entry, "text")?,
                level: number(entry, "level")?,
                rect: rect(&text(entry, "rect")?)?,
                current: entry["current"]
                    .as_bool()
                    .ok_or_else(|| invalid(format!("no current flag: {entry}")))?,
            })
        })
        .collect()
}

/// The document's comments, in kernel order.
pub fn notes(json: &str) -> io::Result<Vec<Note>> {
    let list = parse(json)?;
    entries(&list, "comments")?
        .iter()
        .map(|entry| {
            Ok(Note {
                id: number(entry, "id")?.to_string(),
                parent: number(entry, "parentId")?.to_string(),
                author: text(entry, "author")?,
                text: plain(&text(entry, "html")?),
                resolved: text(entry, "resolved")? == "true",
                at: text(entry, "dateTime")?,
                anchor: rect(&text(entry, "anchorPos")?)?,
            })
        })
        .collect()
}

/// The document's tracked changes, in kernel order.
pub fn changes(json: &str) -> io::Result<Vec<Change>> {
    let list = parse(json)?;
    entries(&list, "redlines")?
        .iter()
        .map(|entry| {
            let range = text(entry, "textRange")?;
            let first = range.split(';').next().unwrap_or_default();
            Ok(Change {
                index: number(entry, "index")?,
                author: text(entry, "author")?,
                kind: text(entry, "type")?,
                description: text(entry, "description")?,
                at: text(entry, "dateTime")?,
                anchor: (!first.trim().is_empty())
                    .then(|| rect(first))
                    .transpose()?,
            })
        })
        .collect()
}

/// A bookmark from `.uno:Bookmarks`.
#[derive(Clone, Debug, PartialEq)]
pub struct Bookmark {
    pub name: String,
    /// None when the layout does not show it.
    pub rect: Option<Rect>,
}

/// The document's bookmarks.
pub fn bookmarks(json: &str) -> io::Result<Vec<Bookmark>> {
    let list = parse(json)?;
    entries(&list, "bookmarks")?
        .iter()
        .map(|entry| {
            Ok(Bookmark {
                name: text(entry, "name")?,
                rect: entry
                    .get("rect")
                    .map(|_| rect(&text(entry, "rect")?))
                    .transpose()?,
            })
        })
        .collect()
}

/// The selected formula from `.uno:Formula`, if one is.
pub fn formula(json: &str) -> io::Result<Option<String>> {
    let list = parse(json)?;
    match &list["formula"] {
        serde_json::Value::Null => Ok(None),
        value => value
            .as_str()
            .map(|formula| Some(formula.to_string()))
            .ok_or_else(|| invalid(format!("a bad formula: {value}"))),
    }
}

pub(crate) fn parse(json: &str) -> io::Result<serde_json::Value> {
    serde_json::from_str(json).map_err(|error| invalid(format!("a bad kernel list: {error}")))
}

pub(crate) fn entries<'a>(list: &'a serde_json::Value, key: &str) -> io::Result<&'a Vec<serde_json::Value>> {
    list[key]
        .as_array()
        .ok_or_else(|| invalid(format!("the list has no {key}")))
}

fn text(entry: &serde_json::Value, key: &str) -> io::Result<String> {
    entry[key]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| invalid(format!("an entry has no {key}: {entry}")))
}

fn number(entry: &serde_json::Value, key: &str) -> io::Result<u64> {
    entry[key]
        .as_u64()
        .ok_or_else(|| invalid(format!("an entry has no {key}: {entry}")))
}

/// "x, y, w, h" in twips.
fn rect(text: &str) -> io::Result<Rect> {
    let parts: Vec<i32> = text
        .split(',')
        .map(|part| part.trim().parse())
        .collect::<Result<_, _>>()
        .map_err(|_| invalid(format!("a bad rectangle: {text}")))?;
    match parts[..] {
        [x, y, w, h] => Ok((x, y, w, h)),
        _ => Err(invalid(format!("a bad rectangle: {text}"))),
    }
}

/// The kernel's simple comment HTML as text.
fn plain(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    // An empty paragraph holds one br.
    let mut broke = false;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        broke &= open == 0;
        let Some(close) = rest[open..].find('>') else {
            break;
        };
        let tag = rest[open + 1..open + close].trim().to_ascii_lowercase();
        if tag.starts_with("br") || ((tag == "/div" || tag == "/p") && !broke) {
            out.push('\n');
        }
        broke = tag.starts_with("br");
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    // The last paragraph's end is no line.
    out.strip_suffix('\n')
        .unwrap_or(&out)
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
}

pub(crate) fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_body_reads_with_its_caret_or_as_none() {
        let body = reading(r#"{"paragraphs":["one","二"],"caret":1}"#).expect("a body");
        assert_eq!(body, Some(Body { paragraphs: vec!["one".into(), "二".into()], caret: 1 }));
        assert_eq!(reading(r#"{"paragraphs":["note"]}"#).expect("no caret"), None, "outside the body");
        assert_eq!(reading("{}").expect("no caret"), None, "text typed into a shape");
        let astray = reading(r#"{"paragraphs":["secret"],"caret":1}"#).expect_err("a caret past the list");
        assert!(!astray.to_string().contains("secret"), "the error leaves the text out: {astray}");
    }

    #[test]
    fn comments_and_changes_parse() {
        let notes = notes(r#"{"comments":[{"id":1,"parentId":0,"author":"A","html":"<div>One &amp; two</div><div>three</div>","resolved":"false","dateTime":"2026-10-04T10:00:00","anchorPos":"4490, 14759, 1, 231","textRange":""}]}"#)
            .expect("notes parse");
        assert_eq!(notes[0].text, "One & two\nthree");
        assert_eq!(
            plain("<div>a</div><div><br/></div><div>Next</div>"),
            "a\n\nNext"
        );
        assert_eq!(plain("<div>a<br/>b</div>"), "a\nb");
        assert_eq!(
            plain("<div>a</div><div><br/></div><div><br/></div>"),
            "a\n\n"
        );
        assert_eq!(notes[0].anchor, (4490, 14759, 1, 231));
        let changes = changes(r#"{"redlines":[{"index":1,"author":"A","type":"Insert","comment":"","description":"Insert x","dateTime":"2026-10-04T10:00:00","textRange":"4490, 14759, 648, 230; 1, 2, 3, 4"}]}"#)
            .expect("changes parse");
        assert_eq!(changes[0].anchor, Some((4490, 14759, 648, 230)));
        assert!(super::notes(r#"{"comments":[{"id":1}]}"#).is_err());
        assert_eq!(formula("{}").expect("empty"), None);
        let counts = counts(r#"{"document":{"words":11,"characters":41,"charactersNoSpaces":31},"selection":{"words":2,"characters":7,"charactersNoSpaces":6}}"#)
            .expect("counts parse");
        assert_eq!((counts.words, counts.selected_characters), (11, 7));
    }
}
