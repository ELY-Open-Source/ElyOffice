//! Links on shown slides, followed by a click (SHOW-11).

use std::{io, path::Path};

use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};

use super::Show;
use crate::deck::Deck;

/// x, y, width, height on the slide, 1/100 mm.
type Rect = (i64, i64, i64, i64);

/// Where a click on a link goes.
#[derive(Clone, Debug, PartialEq)]
pub enum Goto {
    /// A slide, by place in the deck.
    Slide(usize),
    /// A slide the deck does not have.
    Broken(String),
    Next,
    Previous,
    First,
    Last,
    End,
    /// A program, sound or macro; never run.
    Refused,
    /// An address; only web and mail open outside.
    Address(String),
}

/// A shape on a slide that goes somewhere when clicked.
#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    pub bounds: Rect,
    pub to: Goto,
    /// Placed by end points: clicks near it count.
    pub line: bool,
}

/// Impress's hit tolerance (sd `FuPoor::HITPIX`), in screen pixels.
const HIT_PIXELS: f32 = 2.0;

/// A target as named, before slide names are known.
enum Pending {
    Done(Goto),
    Name(String),
}

struct Found {
    link: Link,
    pending: Pending,
    /// Nesting and document order: later is on top.
    rank: (usize, usize),
}

enum Open {
    Page,
    Notes,
    Shape,
    Other,
}

struct Shape {
    name: String,
    line: bool,
    bounds: Option<Rect>,
    /// What its children cover, for a group.
    union: Option<Rect>,
    goes: Vec<Pending>,
    rank: (usize, usize),
}

/// Shapes that take a place on a slide (ODF 19.228).
const SHAPES: [&str; 15] = [
    "draw:frame",
    "draw:custom-shape",
    "draw:rect",
    "draw:ellipse",
    "draw:circle",
    "draw:path",
    "draw:polygon",
    "draw:polyline",
    "draw:regular-polygon",
    "draw:caption",
    "draw:control",
    "draw:line",
    "draw:connector",
    "draw:measure",
    "draw:g",
];

/// The shown copy's links, slide by slide.
pub fn read(copy: &Path) -> io::Result<Vec<Vec<Link>>> {
    let content = ely_office::files::part(copy, "content.xml")?
        .ok_or_else(|| bad("the show's copy has no content.xml"))?;
    parse(&content)
}

fn bad(what: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what.to_string())
}

fn attr(tag: &BytesStart, key: &str) -> io::Result<Option<String>> {
    for found in tag.attributes() {
        let found = found.map_err(bad)?;
        if found.key.as_ref() == key.as_bytes() {
            return Ok(Some(found.normalized_value(quick_xml::XmlVersion::Implicit1_0).map_err(bad)?.into_owned()));
        }
    }
    Ok(None)
}

fn join(a: Option<Rect>, b: Rect) -> Rect {
    let Some(a) = a else {
        return b;
    };
    let (left, top) = (a.0.min(b.0), a.1.min(b.1));
    let (right, bottom) = ((a.0 + a.2).max(b.0 + b.2), (a.1 + a.3).max(b.1 + b.3));
    (left, top, right - left, bottom - top)
}

/// A link's address: a slide by name, or outside.
fn address(href: &str) -> Pending {
    match href.strip_prefix('#') {
        Some(name) => Pending::Name(name.to_string()),
        None => Pending::Done(Goto::Address(href.to_string())),
    }
}

/// A click action that moves about the show, if any.
fn action(tag: &BytesStart) -> io::Result<Option<Pending>> {
    if attr(tag, "script:event-name")?.as_deref() != Some("dom:click") {
        return Ok(None);
    }
    Ok(Some(match attr(tag, "presentation:action")?.as_deref() {
        Some("none") => return Ok(None),
        Some("next-page") => Pending::Done(Goto::Next),
        Some("previous-page") => Pending::Done(Goto::Previous),
        Some("first-page") => Pending::Done(Goto::First),
        Some("last-page") => Pending::Done(Goto::Last),
        Some("stop") => Pending::Done(Goto::End),
        Some("show") => address(&attr(tag, "xlink:href")?.ok_or_else(|| bad("a show action with no slide"))?),
        // Programs, sounds and macros never run.
        other => {
            log::info!("show: a click action {} is refused", other.unwrap_or("script"));
            Pending::Done(Goto::Refused)
        }
    }))
}

#[derive(Default)]
struct Parser {
    open: Vec<Open>,
    shapes: Vec<Shape>,
    names: Vec<String>,
    pages: Vec<Vec<Found>>,
    /// Elements opened inside a notes page.
    hidden: usize,
    in_page: bool,
    order: usize,
}

impl Parser {
    fn start(&mut self, tag: &BytesStart) -> io::Result<()> {
        let name = std::str::from_utf8(tag.name().as_ref()).map_err(bad)?.to_string();
        if self.hidden > 0 {
            self.open.push(Open::Other);
            return Ok(());
        }
        let kind = match name.as_str() {
            "draw:page" if !self.in_page => {
                self.in_page = true;
                self.pages.push(Vec::new());
                self.names.push(attr(tag, "draw:name")?.unwrap_or_default());
                Open::Page
            }
            "presentation:notes" => {
                self.hidden += 1;
                Open::Notes
            }
            shape if self.in_page && SHAPES.contains(&shape) => {
                self.order += 1;
                let rank = (self.shapes.len(), self.order);
                let bounds = place::bounds(shape, tag)?;
                let line = place::by_ends(shape);
                self.shapes.push(Shape { name: shape.to_string(), line, bounds, union: None, goes: Vec::new(), rank });
                Open::Shape
            }
            "text:a" => {
                if let Some(href) = attr(tag, "xlink:href")? {
                    self.goes(&name, address(&href));
                }
                Open::Other
            }
            "presentation:event-listener" | "script:event-listener" => {
                if let Some(go) = action(tag)? {
                    self.goes(&name, go);
                }
                Open::Other
            }
            _ => Open::Other,
        };
        self.open.push(kind);
        Ok(())
    }

    /// A link for the innermost shape; outside one, said.
    fn goes(&mut self, name: &str, go: Pending) {
        match self.shapes.last_mut() {
            Some(shape) => shape.goes.push(go),
            None if self.in_page => log::warn!("show: a {name} outside any shape is dropped"),
            None => {}
        }
    }

    fn end(&mut self) -> io::Result<()> {
        match self.open.pop().ok_or_else(|| bad("an element closed that never opened"))? {
            Open::Page => {
                self.in_page = false;
                if let Some(page) = self.pages.last_mut() {
                    page.sort_by_key(|found| found.rank);
                }
            }
            Open::Notes => self.hidden -= 1,
            Open::Shape => {
                let shape = self.shapes.pop().ok_or_else(|| bad("a shape closed that never opened"))?;
                let Some(bounds) = shape.bounds.or(shape.union) else {
                    if !shape.goes.is_empty() {
                        log::warn!("show: a link on a {} has no place on the slide", shape.name);
                    }
                    return Ok(());
                };
                if let Some(parent) = self.shapes.last_mut() {
                    parent.union = Some(join(parent.union, bounds));
                }
                let page = self.pages.last_mut().ok_or_else(|| bad("a shape outside a slide"))?;
                for pending in shape.goes {
                    let link = Link { bounds, to: Goto::End, line: shape.line };
                    page.push(Found { link, pending, rank: shape.rank });
                }
            }
            Open::Other => {}
        }
        Ok(())
    }
}

/// The links of every slide in `content.xml`, in slide order.
pub fn parse(xml: &[u8]) -> io::Result<Vec<Vec<Link>>> {
    let mut reader = Reader::from_reader(xml);
    let mut parser = Parser::default();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(bad)? {
            Event::Start(tag) => parser.start(&tag)?,
            Event::Empty(tag) => {
                parser.start(&tag)?;
                parser.end()?;
            }
            Event::End(_) => parser.end()?,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    let Parser { names, pages, .. } = parser;
    Ok(pages
        .into_iter()
        .map(|page| {
            page.into_iter()
                .map(|found| {
                    let to = match found.pending {
                        Pending::Done(to) => to,
                        Pending::Name(name) => match names.iter().position(|slide| *slide == name) {
                            Some(at) => Goto::Slide(at),
                            None => Goto::Broken(name),
                        },
                    };
                    Link { to, ..found.link }
                })
                .collect()
        })
        .collect())
}

/// Topmost link under a point; the slide shows `pixels` wide.
pub fn at(links: &[Link], slide: (f32, f32), (x, y): (f32, f32), pixels: f32) -> Option<&Goto> {
    let (x, y) = (x * slide.0, y * slide.1);
    let near = HIT_PIXELS * slide.0 / pixels;
    let inside = |link: &&Link| {
        let (left, top, width, height) = link.bounds;
        let (left, top, right, bottom) = (left as f32, top as f32, (left + width) as f32, (top + height) as f32);
        match link.line {
            true => (left - near..=right + near).contains(&x) && (top - near..=bottom + near).contains(&y),
            false => (left..right).contains(&x) && (top..bottom).contains(&y),
        }
    };
    links.iter().rev().find(inside).map(|link| &link.to)
}

impl Show {
    /// The shown slide's link under a point, as shares.
    pub fn link_at(&self, point: (f32, f32), pixels: f32) -> Option<Goto> {
        at(self.links.get(self.part()?)?, self.slide, point, pixels).cloned()
    }

    /// Does what a click on a link says.
    pub fn follow(&mut self, to: Goto, cx: &mut gpui::Context<Self>) {
        log::info!("show: follows a link");
        match to {
            Goto::Slide(at) => self.jump(at + 1, cx),
            Goto::Next => self.next_slide(cx),
            Goto::Previous => self.previous(cx),
            Goto::First => match self.slides.iter().position(|slide| !slide.hidden) {
                Some(at) => self.jump(at + 1, cx),
                None => log::warn!("show: every slide is hidden"),
            },
            Goto::Last => match self.slides.iter().rposition(|slide| !slide.hidden) {
                Some(at) => self.jump(at + 1, cx),
                None => log::warn!("show: every slide is hidden"),
            },
            Goto::End => self.close(cx),
            Goto::Address(address) => match ely_ui::outward(&address) {
                Ok(address) => cx.open_url(&address),
                Err(notice) => {
                    self.notice = Some(Deck::text(cx, notice, &[]));
                    cx.notify();
                }
            },
            Goto::Refused => {
                self.notice = Some(Deck::text(cx, "office.link.refused", &[]));
                cx.notify();
            }
            Goto::Broken(name) => {
                self.notice = Some(Deck::text(cx, "show.link.broken", &[("name", &name)]));
                cx.notify();
            }
        }
    }
}

mod place;
#[cfg(test)]
mod tests;
