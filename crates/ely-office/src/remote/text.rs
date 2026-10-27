//! The text of a flat ODF document or presentation.

use std::io;

use quick_xml::{
    XmlVersion,
    escape::resolve_xml_entity,
    events::Event,
    name::{Namespace, ResolveResult},
    reader::NsReader,
};

const OFFICE: &[u8] = b"urn:oasis:names:tc:opendocument:xmlns:office:1.0";
const TEXT: &[u8] = b"urn:oasis:names:tc:opendocument:xmlns:text:1.0";
const DRAW: &[u8] = b"urn:oasis:names:tc:opendocument:xmlns:drawing:1.0";
const PRESENTATION: &[u8] = b"urn:oasis:names:tc:opendocument:xmlns:presentation:1.0";
const OFFICEOOO: &[u8] = b"http://openoffice.org/2009/office";

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
    read(bytes, Body::Text)
}

/// Each slide's paragraphs by line; form feeds part slides.
pub fn slide_text(bytes: &[u8]) -> io::Result<String> {
    read(bytes, Body::Slides)
}

fn read(bytes: &[u8], kind: Body) -> io::Result<String> {
    let mut reader = NsReader::from_reader(bytes);
    let (mut buf, mut out) = (Vec::new(), String::new());
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
            Event::Start(tag) if paragraph(tag.local_name().as_ref()) => open += 1,
            Event::Empty(tag) if inside && open == 0 && paragraph(tag.local_name().as_ref()) => out.push('\n'),
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
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{body_text, slide_text};

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
