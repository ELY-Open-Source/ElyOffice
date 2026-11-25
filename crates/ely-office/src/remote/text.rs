//! The text of a flat ODF document or presentation.

use std::{collections::HashMap, io, ops::Range};

use quick_xml::{
    XmlVersion,
    escape::resolve_xml_entity,
    events::{BytesStart, Event},
    name::{Namespace, ResolveResult},
    reader::NsReader,
};

const OFFICE: &[u8] = b"urn:oasis:names:tc:opendocument:xmlns:office:1.0";
const TEXT: &[u8] = b"urn:oasis:names:tc:opendocument:xmlns:text:1.0";
const DRAW: &[u8] = b"urn:oasis:names:tc:opendocument:xmlns:drawing:1.0";
const PRESENTATION: &[u8] = b"urn:oasis:names:tc:opendocument:xmlns:presentation:1.0";
const OFFICEOOO: &[u8] = b"http://openoffice.org/2009/office";
const STYLE: &[u8] = b"urn:oasis:names:tc:opendocument:xmlns:style:1.0";

/// A paragraph's written style and its span in the text.
type Span = (Option<String>, Range<usize>);

/// Writer's body or Impress's slides.
#[derive(Clone, Copy)]
enum Body {
    Text,
    Slides,
}

impl Body {
    fn name(self) -> &'static [u8] {
        match self {
            Body::Text => b"text",
            Body::Slides => b"presentation",
        }
    }

    /// Where Replace All never looks.
    fn left_out(self, ns: &[u8], name: &[u8]) -> bool {
        let comment = (ns == OFFICE || ns == OFFICEOOO) && name == b"annotation";
        match self {
            Body::Text => comment || ns == DRAW || (ns == TEXT && matches!(name, b"note" | b"tracked-changes")),
            Body::Slides => comment || (ns == PRESENTATION && name == b"notes"),
        }
    }
}

fn bad(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("the kernel's text: {error}"))
}

/// Each paragraph and heading of `office:text`, one per line.
pub fn body_text(bytes: &[u8]) -> io::Result<String> {
    read(bytes, Body::Text).map(|(text, _)| text)
}

/// Each body paragraph's style name and text, in order.
pub fn body_paragraphs(bytes: &[u8]) -> io::Result<Vec<(String, String)>> {
    let names = style_names(bytes)?;
    let (text, spans) = read(bytes, Body::Text)?;
    spans
        .into_iter()
        .map(|(style, span)| {
            // ODF: no style named is the default, Writer's Standard.
            let style = match style {
                None => "Standard".to_owned(),
                Some(raw) => names.get(&raw).cloned().ok_or_else(|| bad(format!("no paragraph style {raw}")))?,
            };
            Ok((style, text[span].to_owned()))
        })
        .collect()
}

/// Paragraph styles by written name, as Writer's API names them.
fn style_names(bytes: &[u8]) -> io::Result<HashMap<String, String>> {
    let mut reader = NsReader::from_reader(bytes);
    let (mut buf, mut automatic) = (Vec::new(), false);
    let (mut named, mut parents) = (HashMap::new(), HashMap::new());
    loop {
        let (bound, event) = reader.read_resolved_event_into(&mut buf).map_err(bad)?;
        let ns = match bound {
            ResolveResult::Bound(Namespace(ns)) => ns,
            _ => b"",
        };
        match event {
            Event::Start(tag) if ns == OFFICE && tag.local_name().as_ref() == b"automatic-styles" => automatic = true,
            Event::End(tag) if ns == OFFICE && tag.local_name().as_ref() == b"automatic-styles" => automatic = false,
            Event::Start(tag) if ns == OFFICE && tag.local_name().as_ref() == b"body" => break,
            Event::Start(tag) | Event::Empty(tag) if ns == STYLE && tag.local_name().as_ref() == b"style" => {
                if attribute(&tag, b"family")?.as_deref() == Some("paragraph") {
                    let name = attribute(&tag, b"name")?.ok_or_else(|| bad("a paragraph style without a name"))?;
                    if automatic {
                        parents.insert(name, attribute(&tag, b"parent-style-name")?);
                    } else {
                        let shown = attribute(&tag, b"display-name")?.unwrap_or_else(|| name.clone());
                        named.insert(name, shown);
                    }
                }
            }
            Event::Eof => return Err(bad("no document body")),
            _ => {}
        }
        buf.clear();
    }
    for (name, parent) in parents {
        // An automatic style without a parent sits on Standard.
        let shown = match parent {
            None => "Standard".to_owned(),
            Some(parent) => named.get(&parent).cloned().ok_or_else(|| bad(format!("no paragraph style {parent}")))?,
        };
        named.insert(name, shown);
    }
    Ok(named)
}

/// The value of `tag`'s attribute named `key`, any namespace.
fn attribute(tag: &BytesStart, key: &[u8]) -> io::Result<Option<String>> {
    let found = tag.attributes().filter_map(Result::ok).find(|attr| attr.key.local_name().as_ref() == key);
    found.map(|attr| attr.normalized_value(XmlVersion::Explicit1_0).map(|value| value.into_owned()).map_err(bad)).transpose()
}

/// Each slide's paragraphs by line; form feeds part slides.
pub fn slide_text(bytes: &[u8]) -> io::Result<String> {
    read(bytes, Body::Slides).map(|(text, _)| text)
}

fn read(bytes: &[u8], kind: Body) -> io::Result<(String, Vec<Span>)> {
    let mut reader = NsReader::from_reader(bytes);
    let (mut buf, mut out) = (Vec::new(), String::new());
    let (mut starts, mut spans) = (Vec::<(Option<String>, usize)>::new(), Vec::<Span>::new());
    let (mut body, mut skipped, mut open, mut slides) = (false, 0usize, 0usize, 0usize);
    loop {
        let (bound, event) = reader.read_resolved_event_into(&mut buf).map_err(bad)?;
        let ns = match bound {
            ResolveResult::Bound(Namespace(ns)) => ns,
            _ => b"",
        };
        let inside = body && skipped == 0;
        let paragraph = |name: &[u8]| ns == TEXT && matches!(name, b"p" | b"h");
        match event {
            Event::Start(_) if skipped > 0 => skipped += 1,
            Event::Start(tag) if !body => body = ns == OFFICE && tag.local_name().as_ref() == kind.name(),
            Event::Start(tag) if kind.left_out(ns, tag.local_name().as_ref()) => skipped = 1,
            Event::Start(tag) | Event::Empty(tag) if inside && ns == DRAW && tag.local_name().as_ref() == b"page" => {
                slides += 1;
                if slides > 1 {
                    if out.ends_with('\n') {
                        out.pop();
                    }
                    out.push('\x0c');
                }
            }
            Event::Start(tag) if paragraph(tag.local_name().as_ref()) => {
                open += 1;
                starts.push((attribute(&tag, b"style-name")?, out.len()));
            }
            Event::Empty(tag) if inside && open == 0 && paragraph(tag.local_name().as_ref()) => {
                spans.push((attribute(&tag, b"style-name")?, out.len()..out.len()));
                out.push('\n');
            }
            Event::Empty(tag) if inside && open > 0 && ns == TEXT => match tag.local_name().as_ref() {
                b"s" => {
                    let count = tag.attributes().filter_map(Result::ok).find(|attr| attr.key.local_name().as_ref() == b"c");
                    let count = match count {
                        Some(attr) => attr.normalized_value(XmlVersion::Explicit1_0).map_err(bad)?.parse::<usize>().map_err(bad)?,
                        None => 1,
                    };
                    out.extend(std::iter::repeat_n(' ', count));
                }
                b"tab" => out.push('\t'),
                b"line-break" => out.push('\n'),
                _ => {}
            },
            Event::End(_) if skipped > 0 => skipped -= 1,
            Event::End(tag) if inside && paragraph(tag.local_name().as_ref()) => {
                open -= 1;
                let (style, start) = starts.pop().ok_or_else(|| bad("a paragraph closed twice"))?;
                spans.push((style, start..out.len()));
                out.push('\n');
            }
            Event::End(tag) if inside && ns == OFFICE && tag.local_name().as_ref() == kind.name() => break,
            Event::Text(text) if inside && open > 0 => out.push_str(&text.xml10_content().map_err(bad)?),
            Event::GeneralRef(entity) if inside && open > 0 => match entity.resolve_char_ref().map_err(bad)? {
                Some(char) => out.push(char),
                None => {
                    let name = entity.decode().map_err(bad)?;
                    out.push_str(resolve_xml_entity(&name).ok_or_else(|| bad(format!("&{name};")))?);
                }
            },
            Event::Eof => return Err(bad("no document body")),
            _ => {}
        }
        buf.clear();
    }
    if out.ends_with('\n') {
        out.pop();
    }
    Ok((out, spans))
}

#[cfg(test)]
mod tests {
    use super::{body_paragraphs, body_text, slide_text};

    fn doc(body: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"><office:body><office:text>
 <text:sequence-decls><text:sequence-decl text:name="Table"/></text:sequence-decls>
 {body}
</office:text></office:body></office:document>"#
        )
    }

    #[test]
    fn paragraphs_come_one_per_line() {
        let body = r#"<text:h text:outline-level="1">Title</text:h><text:p>a<text:s/>b<text:s text:c="3"/>c<text:tab/>d<text:line-break/>e &amp; &#x1F600; 世界</text:p><text:p/><text:p><text:span>span</text:span></text:p>"#;
        assert_eq!(body_text(doc(body).as_bytes()).expect("reads"), "Title\na b   c\td\ne & 😀 世界\n\nspan");
    }

    #[test]
    fn tables_and_lists_count_as_paragraphs() {
        let body = r#"<table:table><table:table-row><table:table-cell><text:p>A1</text:p></table:table-cell><table:table-cell><text:p>B1</text:p></table:table-cell></table:table-row></table:table><text:list><text:list-item><text:p>item</text:p></text:list-item></text:list>"#;
        assert_eq!(body_text(doc(body).as_bytes()).expect("reads"), "A1\nB1\nitem");
    }

    #[test]
    fn notes_comments_frames_and_deletions_stay_out() {
        let body = r#"<text:tracked-changes><text:changed-region><text:deletion><text:p>gone</text:p></text:deletion></text:changed-region></text:tracked-changes><text:p>see<text:note><text:note-citation>1</text:note-citation><text:note-body><text:p>note</text:p></text:note-body></text:note><office:annotation><text:p>comment</text:p></office:annotation><draw:frame><draw:text-box><text:p>boxed</text:p></draw:text-box></draw:frame> here</text:p>"#;
        assert_eq!(body_text(doc(body).as_bytes()).expect("reads"), "see here");
    }

    #[test]
    fn paragraphs_carry_their_style_by_its_api_name() {
        let styled = |body: &str| {
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?><office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"><office:styles><style:style style:name="Standard" style:family="paragraph"/><style:style style:name="Heading_20_1" style:display-name="Heading 1" style:family="paragraph"/><style:style style:name="Text_20_body" style:display-name="Text body" style:family="paragraph"/><style:style style:name="Strong" style:family="text"/></office:styles><office:automatic-styles><style:style style:name="P1" style:family="paragraph" style:parent-style-name="Text_20_body"/><style:style style:name="P2" style:family="paragraph"/></office:automatic-styles><office:body><office:text>{body}</office:text></office:body></office:document>"#
            )
        };
        let body = r#"<text:h text:style-name="Heading_20_1">Title</text:h><text:p text:style-name="P1">a<text:line-break/>b</text:p><text:p>bare</text:p><text:p text:style-name="P2">auto</text:p><text:p text:style-name="Standard"/><text:list><text:list-item><text:p text:style-name="Text_20_body">item</text:p></text:list-item></text:list>"#;
        let got = body_paragraphs(styled(body).as_bytes()).expect("reads");
        let want = [("Heading 1", "Title"), ("Text body", "a\nb"), ("Standard", "bare"), ("Standard", "auto"), ("Standard", ""), ("Text body", "item")];
        assert_eq!(got, want.map(|(style, text)| (style.to_owned(), text.to_owned())));
        assert!(body_paragraphs(styled(r#"<text:p text:style-name="Gone">x</text:p>"#).as_bytes()).is_err(), "an unknown style");
        assert_eq!(body_text(styled(body).as_bytes()).expect("reads"), "Title\na\nb\nbare\nauto\n\nitem", "plain get is unchanged");
    }

    #[test]
    fn a_document_without_a_text_body_is_refused() {
        assert!(body_text(b"<office:document/>").is_err());
        assert!(body_text(b"not xml <").is_err());
        assert!(slide_text(doc("<text:p>a</text:p>").as_bytes()).is_err(), "text is no deck");
    }

    #[test]
    fn slides_part_by_form_feeds_without_notes_or_comments() {
        let slides = r#"<draw:page draw:name="1"><draw:frame><draw:text-box><text:p>Title</text:p><text:p>a<text:tab/>b</text:p></draw:text-box></draw:frame><presentation:notes><draw:frame><draw:text-box><text:p>note</text:p></draw:text-box></draw:frame></presentation:notes></draw:page><draw:page draw:name="2"/><draw:page draw:name="3"><draw:custom-shape><text:p>shape</text:p></draw:custom-shape><draw:frame><table:table><table:table-row><table:table-cell><text:p>cell</text:p></table:table-cell></table:table-row></table:table></draw:frame><draw:g><draw:frame><draw:text-box><text:p>grouped</text:p></draw:text-box></draw:frame></draw:g><officeooo:annotation><text:p>comment</text:p></officeooo:annotation></draw:page>"#;
        let deck = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:presentation="urn:oasis:names:tc:opendocument:xmlns:presentation:1.0" xmlns:officeooo="http://openoffice.org/2009/office"><office:body><office:presentation>{slides}</office:presentation></office:body></office:document>"#
        );
        assert_eq!(slide_text(deck.as_bytes()).expect("reads"), "Title\na\tb\x0c\x0cshape\ncell\ngrouped");
    }
}
