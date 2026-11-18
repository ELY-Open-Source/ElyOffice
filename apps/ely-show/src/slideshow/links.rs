//! Links on the shown slides, read from the show's copy and followed by a click (SHOW-11).

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
    /// An address: opened outside only if it is web or mail.
    Address(String),
}

/// A shape on a slide that goes somewhere when clicked.
#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    pub bounds: Rect,
    pub to: Goto,
}

/// A target as the XML names it, before slide names are known.
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
    bounds: Option<Rect>,
    /// What its children cover, for a group.
    union: Option<Rect>,
    goes: Vec<Pending>,
    rank: (usize, usize),
}

/// The elements that take a place on the slide.
const SHAPES: [&str; 9] = [
    "draw:frame",
    "draw:custom-shape",
    "draw:rect",
    "draw:ellipse",
    "draw:circle",
    "draw:path",
    "draw:polygon",
    "draw:caption",
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

/// A length such as `2.5cm` in 1/100 mm.
fn length(text: &str) -> io::Result<i64> {
    let at = text
        .find(|c: char| c.is_alphabetic())
        .ok_or_else(|| bad(format!("a length with no unit: {text}")))?;
    let (number, unit) = text.split_at(at);
    let number: f64 = number.parse().map_err(|_| bad(format!("a bad length: {text}")))?;
    let mm = match unit {
        "cm" => 10.0,
        "mm" => 1.0,
        "in" => 25.4,
        "pt" => 25.4 / 72.0,
        "pc" => 25.4 / 6.0,
        "px" => 25.4 / 96.0,
        _ => return Err(bad(format!("a length in {unit}: {text}"))),
    };
    Ok((number * mm * 100.0).round() as i64)
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

fn rect(tag: &BytesStart) -> io::Result<Option<Rect>> {
    let mut parts = [None; 4];
    for (part, key) in parts.iter_mut().zip(["svg:x", "svg:y", "svg:width", "svg:height"]) {
        *part = attr(tag, key)?.map(|text| length(&text)).transpose()?;
    }
    Ok(match parts {
        [Some(x), Some(y), Some(w), Some(h)] => Some((x, y, w, h)),
        _ => None,
    })
}

fn join(a: Option<Rect>, b: Rect) -> Rect {
    let Some(a) = a else {
        return b;
    };
    let (left, top) = (a.0.min(b.0), a.1.min(b.1));
    let (right, bottom) = ((a.0 + a.2).max(b.0 + b.2), (a.1 + a.3).max(b.1 + b.3));
    (left, top, right - left, bottom - top)
}

/// Where an address in a link goes: a slide by name, or outside.
fn address(href: &str) -> Pending {
    match href.strip_prefix('#') {
        Some(name) => Pending::Name(name.to_string()),
        None => Pending::Done(Goto::Address(href.to_string())),
    }
}

/// What a click action does; the ones that are not moving about are left out.
fn action(tag: &BytesStart) -> io::Result<Option<Pending>> {
    if attr(tag, "script:event-name")?.as_deref() != Some("dom:click") {
        return Ok(None);
    }
    let Some(action) = attr(tag, "presentation:action")? else {
        return Ok(None);
    };
    Ok(Some(match action.as_str() {
        "next-page" => Pending::Done(Goto::Next),
        "previous-page" => Pending::Done(Goto::Previous),
        "first-page" => Pending::Done(Goto::First),
        "last-page" => Pending::Done(Goto::Last),
        "stop" => Pending::Done(Goto::End),
        "show" => address(&attr(tag, "xlink:href")?.ok_or_else(|| bad("a show action with no slide"))?),
        // Programs, sounds and the like are not run from a slide.
        other => {
            log::info!("show: a click action {other} is not followed");
            return Ok(None);
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
                self.shapes.push(Shape { bounds: rect(tag)?, union: None, goes: Vec::new(), rank });
                Open::Shape
            }
            "text:a" => {
                if let (Some(shape), Some(href)) = (self.shapes.last_mut(), attr(tag, "xlink:href")?) {
                    shape.goes.push(address(&href));
                }
                Open::Other
            }
            "presentation:event-listener" => {
                if let (Some(shape), Some(go)) = (self.shapes.last_mut(), action(tag)?) {
                    shape.goes.push(go);
                }
                Open::Other
            }
            _ => Open::Other,
        };
        self.open.push(kind);
        Ok(())
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
                    return Ok(());
                };
                if let Some(parent) = self.shapes.last_mut() {
                    parent.union = Some(join(parent.union, bounds));
                }
                let page = self.pages.last_mut().ok_or_else(|| bad("a shape outside a slide"))?;
                for pending in shape.goes {
                    let link = Link { bounds, to: Goto::End };
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

/// The topmost link under a point, given as shares of the slide.
pub fn at(links: &[Link], slide: (f32, f32), (x, y): (f32, f32)) -> Option<&Goto> {
    let (x, y) = (x * slide.0, y * slide.1);
    let inside = |link: &&Link| {
        let (left, top, width, height) = link.bounds;
        (left as f32..(left + width) as f32).contains(&x) && (top as f32..(top + height) as f32).contains(&y)
    };
    links.iter().rev().find(inside).map(|link| &link.to)
}

impl Show {
    /// The shown slide's link under a point, as shares.
    pub fn link_at(&self, point: (f32, f32)) -> Option<Goto> {
        at(self.links.get(self.part()?)?, self.slide, point).cloned()
    }

    /// Does what a click on a link says.
    pub fn follow(&mut self, to: Goto, cx: &mut gpui::Context<Self>) {
        log::info!("show: follows a link");
        match to {
            Goto::Slide(at) => self.jump(at + 1, cx),
            Goto::Next => self.next(cx),
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
            Goto::Address(address) if ely_office::outward(&address) => cx.open_url(&address),
            Goto::Address(_) => {
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

#[cfg(test)]
mod tests;
