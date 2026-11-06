//! A body as safe blocks for the reading pane (§12.4).

use std::ops::Range;

use crate::{Part, Shown};

mod build;
mod plain;
mod text;

/// Quotes and lists deeper than this flatten.
const NESTED: usize = 16;

/// How a stretch of text prints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Strong,
    Emphasis,
    Strike,
    Underline,
    Code,
}

/// Text, its styled stretches and its links.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Inline {
    pub text: String,
    pub styles: Vec<(Range<usize>, Style)>,
    /// Only http, https and mailto addresses.
    pub links: Vec<(Range<usize>, String)>,
}

/// Where a picture's bytes are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// An index into `Mail::inline`.
    Inline(usize),
    /// Bytes a `data:` address carried.
    Data {
        content_type: String,
        bytes: Vec<u8>,
    },
    /// Fetched only once the reader allows.
    Remote(String),
    /// The address names nothing the mail holds.
    Broken,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub alt: String,
    pub source: Source,
    pub link: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Text(Inline),
    Heading(u8, Inline),
    Quote(Vec<Block>),
    /// Items; an ordered list counts from `start`.
    List {
        start: Option<u64>,
        items: Vec<Vec<Block>>,
    },
    Code(String),
    Rule,
    Image(Image),
}

/// What the pane draws, and what it holds back.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Page {
    pub blocks: Vec<Block>,
    /// Remote pictures shown, each address once.
    pub remote: Vec<String>,
    /// Remote pictures never fetched: pixels and hidden ones.
    pub trackers: usize,
    /// The HTML nested too deep and was cut.
    pub short: bool,
}

/// HTML with no script, style or remote load.
pub fn html(html: &str, inline: &[Part]) -> Page {
    build::page(html, inline)
}

/// Every body in order, as one page.
pub fn read(shown: &[Shown], inline: &[Part]) -> Page {
    let mut page = Page::default();
    for body in shown {
        let next = match body {
            Shown::Html(markup) => html(markup, inline),
            Shown::Text(text) => plain(text),
        };
        page.blocks.extend(next.blocks);
        for url in next.remote {
            if !page.remote.contains(&url) {
                page.remote.push(url);
            }
        }
        page.trackers += next.trackers;
        page.short |= next.short;
    }
    page
}

impl Page {
    /// First `most` characters on one line; quotes only alone.
    pub fn snippet(&self, most: usize) -> String {
        let mut words = String::new();
        snip(&self.blocks, false, &mut words);
        if words.trim().is_empty() {
            snip(&self.blocks, true, &mut words);
        }
        let line = words.split_whitespace().collect::<Vec<_>>().join(" ");
        line.chars().take(most).collect()
    }
}

fn snip(blocks: &[Block], quotes: bool, words: &mut String) {
    for block in blocks {
        match block {
            Block::Text(inline) | Block::Heading(_, inline) => words.push_str(&inline.text),
            Block::Code(text) => words.push_str(text),
            Block::Quote(inner) if quotes => snip(inner, quotes, words),
            Block::List { items, .. } => items.iter().for_each(|item| snip(item, quotes, words)),
            Block::Quote(_) | Block::Rule | Block::Image(_) => {}
        }
        words.push(' ');
    }
}

pub fn plain(text: &str) -> Page {
    Page {
        blocks: plain::blocks(text),
        ..Page::default()
    }
}

impl Inline {
    /// Appends `text`, `styles` and `link` over it.
    fn push(&mut self, text: &str, styles: &[Style], link: Option<&str>) {
        let start = self.text.len();
        self.text.push_str(text);
        let end = self.text.len();
        if start == end {
            return;
        }
        for style in styles {
            match self.styles.last_mut() {
                Some((range, last)) if range.end == start && last == style => range.end = end,
                _ => self.styles.push((start..end, *style)),
            }
        }
        if let Some(link) = link {
            match self.links.last_mut() {
                Some((range, last)) if range.end == start && last == link => range.end = end,
                _ => self.links.push((start..end, link.to_owned())),
            }
        }
    }

    /// Cuts white space at both ends.
    fn trim(&mut self) {
        let end = self.text.trim_end().len();
        let start = end - self.text[..end].trim_start().len();
        self.keep(start, end);
    }

    fn trim_end(&mut self) {
        self.keep(0, self.text.trim_end().len());
    }

    /// Keeps bytes `start..end`; spans move with them.
    fn keep(&mut self, start: usize, end: usize) {
        self.text.truncate(end);
        self.text.drain(..start);
        self.styles = within(std::mem::take(&mut self.styles), start, end);
        self.links = within(std::mem::take(&mut self.links), start, end);
    }
}

fn within<T>(ranges: Vec<(Range<usize>, T)>, start: usize, end: usize) -> Vec<(Range<usize>, T)> {
    let clamp = |at: usize| at.clamp(start, end) - start;
    ranges
        .into_iter()
        .map(|(range, value)| (clamp(range.start)..clamp(range.end), value))
        .filter(|(range, _)| !range.is_empty())
        .collect()
}

/// Browsers drop these from an address.
pub fn link(href: &str) -> Option<String> {
    let cut = |ch: char| ch <= ' ';
    let href: String = href
        .trim_matches(cut)
        .chars()
        .filter(|ch| !matches!(ch, '\t' | '\n' | '\r'))
        .collect();
    let lower = href.to_ascii_lowercase();
    ["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme))
        .then_some(href)
}
