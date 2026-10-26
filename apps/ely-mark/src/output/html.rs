//! Markdown as HTML, as the drawing reads it.

use std::{
    cmp::Reverse,
    collections::{HashSet, VecDeque},
    io,
    ops::Range,
};

use ely_gpui_component::documents::{
    AlertKind, Inline, MarkdownBlock, MarkdownReader, Style, alert, diagram_svg, formula_svg,
    is_toc,
    mdast::{AlignKind, Node},
};

use super::{Page, address::safe, attribute, escape, pictures::Placed};

/// Writes `tree`, read from `source`, into `page`.
pub(super) fn write(
    tree: &Node,
    source: &str,
    placed: &mut Placed,
    page: &mut Page,
) -> io::Result<()> {
    let mut writer = Writer {
        placed,
        page,
        source,
        reader: MarkdownReader::joined(source, tree),
        anchored: HashSet::new(),
        lines: line_starts(source),
    };
    writer.root(tree)
}

struct Writer<'a, 'b, 'c> {
    placed: &'b mut Placed<'a>,
    page: &'c mut Page,
    source: &'c str,
    reader: MarkdownReader<'c>,
    /// Notes whose definition has its anchor.
    anchored: HashSet<usize>,
    /// Where each source line starts.
    lines: Vec<usize>,
}

/// What a read area leaves out, in reading order.
#[derive(Default)]
struct Atoms {
    /// Each formula's line.
    formulas: VecDeque<usize>,
    /// Each footnote reference's number.
    notes: VecDeque<usize>,
}

/// A stretch of read text's mark.
enum Mark<'a> {
    Style(Style),
    Link(&'a str),
}

impl Writer<'_, '_, '_> {
    /// The line `node` starts on, from one.
    fn line_of(&self, node: &Node) -> usize {
        let at = node.position().start;
        self.lines.partition_point(|start| *start <= at)
    }

    fn push(&mut self, text: &str) {
        self.page.html.push_str(text);
    }

    /// Root blocks: alerts and `[toc]` live only here.
    fn root(&mut self, tree: &Node) -> io::Result<()> {
        for child in tree.children().into_iter().flatten() {
            match child {
                Node::Blockquote(quote) => match alert(quote) {
                    Some((kind, body)) => self.alert(kind, &body)?,
                    None => self.block(child, false)?,
                },
                _ if is_toc(child, self.source) => self.toc(),
                _ => self.block(child, false)?,
            }
        }
        Ok(())
    }

    fn blocks(&mut self, node: &Node, tight: bool) -> io::Result<()> {
        for child in node.children().into_iter().flatten() {
            self.block(child, tight)?;
        }
        Ok(())
    }

    fn block(&mut self, node: &Node, tight: bool) -> io::Result<()> {
        match node {
            Node::Paragraph(_) | Node::Heading(_) | Node::Table(_) | Node::Html(_) => {
                self.area(node, tight)?
            }
            Node::Blockquote(_) => {
                self.push("<blockquote>");
                self.blocks(node, false)?;
                self.push("</blockquote>");
            }
            Node::List(list) => {
                let tag = if list.ordered { "ol" } else { "ul" };
                match list.start.filter(|start| list.ordered && *start != 1) {
                    Some(start) => self.push(&format!("<ol start=\"{start}\">")),
                    None => self.push(&format!("<{tag}>")),
                }
                for item in &list.children {
                    let Node::ListItem(each) = item else { continue };
                    self.push("<li>");
                    if let Some(checked) = each.checked {
                        let mark = if checked { " checked" } else { "" };
                        self.push(&format!("<input type=\"checkbox\" disabled{mark}> "));
                    }
                    self.blocks(item, !list.spread)?;
                    self.push("</li>");
                }
                self.push(&format!("</{tag}>"));
            }
            Node::Code(code) if code.lang.as_deref() == Some("mermaid") => {
                match diagram_svg(&code.value) {
                    Ok(svg) => self.push(&format!("<div class=\"diagram\">{}</div>", bare(&svg))),
                    Err(_) => {
                        self.page.diagrams.push(self.line_of(node));
                        self.code(Some("mermaid"), &code.value);
                    }
                }
            }
            Node::Code(code) => self.code(code.lang.as_deref(), &code.value),
            Node::Math(math) => self.math(self.line_of(node), &math.value, true),
            Node::ThematicBreak(_) => self.push("<hr>"),
            Node::FootnoteDefinition(note) => {
                // A reference reaches its note's first definition.
                let id = (self.reader.note(&note.identifier))
                    .filter(|number| self.anchored.insert(*number))
                    .map_or_else(String::new, |number| format!(" id=\"fn-{number}\""));
                self.push(&format!(
                    "<div{id} style=\"display:flex;gap:.5em;font-size:.9em;color:#59636e\"><span>[{}]</span><div>",
                    escape(&self.reader.label(note))
                ));
                self.blocks(node, false)?;
                self.push("</div></div>");
            }
            // Never written.
            Node::Yaml(_) | Node::Toml(_) | Node::Definition(_) => {}
            other => unreachable!("{other:?} is no block outside MDX"),
        }
        Ok(())
    }

    /// A paragraph, heading, table or HTML block, read as drawn.
    fn area(&mut self, node: &Node, tight: bool) -> io::Result<()> {
        let mut read = Vec::new();
        self.reader.block(node, &mut read);
        if let Node::Table(_) = node {
            let [MarkdownBlock::Table { aligns, head, rows }] = read.as_slice() else {
                unreachable!("a table reads as one table");
            };
            return self.table(node, aligns, head, rows);
        }
        // A heading's pictures follow it whole.
        let split = !matches!(node, Node::Heading(_));
        let stretches = self.stretches(node, split);
        self.run(&read, node, tight, split, stretches)
    }

    /// Pieces in order; with `split`, each picture ends a stretch.
    fn run(
        &mut self,
        pieces: &[MarkdownBlock],
        node: &Node,
        tight: bool,
        split: bool,
        mut stretches: VecDeque<Atoms>,
    ) -> io::Result<()> {
        let mut written = false;
        for piece in pieces {
            match piece {
                MarkdownBlock::Image { alt, url, link, .. } => {
                    self.picture(url, alt, link.as_deref())?;
                    if split {
                        retire(stretches.pop_front(), written);
                        written = false;
                    }
                }
                _ => {
                    let atoms = stretches.front_mut().expect("a stretch per text");
                    self.piece(piece, node, tight, atoms);
                    written = true;
                }
            }
        }
        retire(stretches.pop_front(), written);
        assert!(stretches.is_empty(), "a stretch per picture");
        Ok(())
    }

    /// Formulas and notes by stretch; pictures part them.
    fn stretches(&self, node: &Node, split: bool) -> VecDeque<Atoms> {
        let mut out = VecDeque::from([Atoms::default()]);
        self.gather(node, split, &mut out);
        out
    }

    fn gather(&self, node: &Node, split: bool, out: &mut VecDeque<Atoms>) {
        match node {
            Node::InlineMath(_) => {
                (out.back_mut().expect("a stretch").formulas).push_back(self.line_of(node))
            }
            Node::FootnoteReference(call) => {
                let number = self
                    .reader
                    .note(&call.identifier)
                    .expect("a reference names a note");
                out.back_mut().expect("a stretch").notes.push_back(number);
            }
            Node::Image(_) | Node::ImageReference(_) if split => out.push_back(Atoms::default()),
            _ => {}
        }
        for child in node.children().into_iter().flatten() {
            self.gather(child, split, out);
        }
    }

    fn table(
        &mut self,
        node: &Node,
        aligns: &[AlignKind],
        head: &[Vec<MarkdownBlock>],
        rows: &[Vec<Vec<MarkdownBlock>>],
    ) -> io::Result<()> {
        let written = node.children().expect("a table holds rows");
        let read: Vec<&[Vec<MarkdownBlock>]> = std::iter::once(head)
            .chain(rows.iter().map(Vec::as_slice))
            .collect();
        assert_eq!(read.len(), written.len(), "a read row per row");
        self.push("<table>");
        for (at, (row, cells)) in read.into_iter().zip(written).enumerate() {
            let tag = if at == 0 { "th" } else { "td" };
            // Cells past the head stay undrawn.
            let cells = &cells.children().expect("a row holds cells");
            let cells = &cells[..cells.len().min(aligns.len())];
            assert_eq!(row.len(), cells.len(), "a read cell per cell");
            self.push("<tr>");
            for (column, (pieces, cell)) in row.iter().zip(cells).enumerate() {
                let side = match aligns.get(column) {
                    Some(AlignKind::Left) => " style=\"text-align:left\"",
                    Some(AlignKind::Right) => " style=\"text-align:right\"",
                    Some(AlignKind::Center) => " style=\"text-align:center\"",
                    _ => "",
                };
                self.push(&format!("<{tag}{side}>"));
                let stretches = self.stretches(cell, true);
                self.run(pieces, cell, true, true, stretches)?;
                self.push(&format!("</{tag}>"));
            }
            self.push("</tr>");
        }
        self.push("</table>");
        Ok(())
    }

    fn piece(&mut self, piece: &MarkdownBlock, node: &Node, tight: bool, atoms: &mut Atoms) {
        match piece {
            MarkdownBlock::Paragraph(text) if tight => self.inline(text, atoms),
            MarkdownBlock::Paragraph(text) => {
                self.push("<p>");
                self.inline(text, atoms);
                self.push("</p>");
            }
            MarkdownBlock::Heading(depth, text) => {
                let at = node.position().start;
                self.push(&format!("<h{depth} id=\"h-{at}\">"));
                self.inline(text, atoms);
                self.push(&format!("</h{depth}>"));
            }
            MarkdownBlock::Math(tex) => {
                let line = atoms.formulas.pop_front().expect("a formula's line");
                self.math(line, tex, true);
            }
            other => unreachable!("an area's text reads as text or math, not {other:?}"),
        }
    }

    /// Read text; its marks open and close as a stack.
    fn inline(&mut self, read: &Inline, atoms: &mut Atoms) {
        let mut marks: Vec<(&Range<usize>, Mark)> = (read.styles.iter())
            .filter(|(_, style)| *style != Style::Link)
            .map(|(range, style)| (range, Mark::Style(*style)))
            .chain(
                read.links
                    .iter()
                    .map(|(range, url)| (range, Mark::Link(url))),
            )
            .collect();
        // Outer first; a formula or note is its own text.
        marks.sort_by_key(|(range, mark)| {
            let whole = matches!(mark, Mark::Style(Style::Math | Style::Footnote));
            (range.start, Reverse(range.end), whole)
        });
        let (mut open, mut at): (Vec<(usize, &str)>, usize) = (Vec::new(), 0);
        let mut links = 0;
        for (range, mark) in marks {
            links -= self.close_to(read, range.start, &mut open, &mut at);
            assert!(
                range.start >= at && open.last().is_none_or(|(end, _)| range.end <= *end),
                "read marks nest"
            );
            self.text(&read.text[at..range.start]);
            at = range.start;
            match mark {
                Mark::Style(Style::Math) => {
                    let line = atoms.formulas.pop_front().expect("a formula's line");
                    self.math(line, &read.text[range.clone()], false);
                    at = range.end;
                }
                Mark::Style(Style::Footnote) => {
                    let number = atoms.notes.pop_front().expect("a note's number");
                    // Links never nest.
                    match links > 0 {
                        true => self.push(&format!("<sup>{number}</sup>")),
                        false => {
                            self.push(&format!("<sup><a href=\"#fn-{number}\">{number}</a></sup>"))
                        }
                    }
                    at = range.end;
                }
                Mark::Style(style) => {
                    let name = tag(style);
                    self.push(&format!("<{name}>"));
                    open.push((range.end, name));
                }
                Mark::Link(url) => {
                    match safe(url) {
                        true => self.push(&format!("<a href=\"{}\">", attribute(url))),
                        false => self.push("<a>"),
                    }
                    open.push((range.end, "a"));
                    links += 1;
                }
            }
        }
        self.close_to(read, read.text.len(), &mut open, &mut at);
        self.text(&read.text[at..]);
    }

    /// Closes each mark ending by `to`; counts closed links.
    fn close_to(
        &mut self,
        read: &Inline,
        to: usize,
        open: &mut Vec<(usize, &str)>,
        at: &mut usize,
    ) -> usize {
        let mut links = 0;
        while let Some(&(end, name)) = open.last().filter(|(end, _)| *end <= to) {
            self.text(&read.text[*at..end]);
            self.push(&format!("</{name}>"));
            *at = end;
            open.pop();
            links += usize::from(name == "a");
        }
        links
    }

    fn text(&mut self, text: &str) {
        self.push(&escape(text).replace('\n', "<br>"));
    }

    fn code(&mut self, lang: Option<&str>, value: &str) {
        let class = lang.map_or_else(String::new, |lang| {
            format!(" class=\"language-{}\"", attribute(lang))
        });
        self.push(&format!("<pre><code{class}>{}</code></pre>", escape(value)));
    }

    fn math(&mut self, line: usize, value: &str, display: bool) {
        match (formula_svg(value, display), display) {
            (Ok(svg), true) => self.push(&format!("<div class=\"math\">{}</div>", bare(&svg))),
            (Ok(svg), false) => self.push(&format!("<span class=\"math\">{}</span>", bare(&svg))),
            (Err(error), _) => {
                self.page.formulas.push((line, error.to_string()));
                match display {
                    true => self.code(Some("math"), value),
                    false => self.push(&format!("<code>{}</code>", escape(value))),
                }
            }
        }
    }

    /// A GitHub alert: its kind's title, then its blocks.
    fn alert(&mut self, kind: AlertKind, body: &[Node]) -> io::Result<()> {
        let (title, hue) = (kind.title(), hue(kind));
        self.push(&format!(
            "<div style=\"padding:.25em 1em;margin:1em 0;border-left:.25em solid {hue}\"><p style=\"font-weight:600;color:{hue}\">{title}</p>"
        ));
        for node in body {
            self.block(node, false)?;
        }
        self.push("</div>");
        Ok(())
    }

    /// `[toc]`: each heading, linked to its place.
    fn toc(&mut self) {
        let items: String = (self.reader.headings.iter())
            .map(|heading| {
                format!(
                    "<li style=\"margin-left:{}em\"><a href=\"#h-{}\">{}</a></li>",
                    heading.depth - 1,
                    heading.at,
                    escape(&heading.title)
                )
            })
            .collect();
        self.push(&format!(
            "<ul style=\"list-style:none;padding-left:0\">{items}</ul>"
        ));
    }

    fn picture(&mut self, url: &str, alt: &str, link: Option<&str>) -> io::Result<()> {
        let src = self.placed.place(url)?;
        if src.is_none() {
            self.page.missing.push(if url.is_empty() {
                alt.to_owned()
            } else {
                url.to_owned()
            });
        }
        // A PDF shows what it left out as words.
        let Some(src) = src.or_else(|| (!self.placed.pdf()).then(|| url.to_owned())) else {
            self.push(&format!("[{}]", escape(alt)));
            return Ok(());
        };
        let image = format!(
            "<img src=\"{}\" alt=\"{}\">",
            attribute(&src),
            attribute(alt)
        );
        match link {
            Some(link) if safe(link) => {
                self.push(&format!("<a href=\"{}\">{image}</a>", attribute(link)))
            }
            Some(_) => self.push(&format!("<a>{image}</a>")),
            None => self.push(&image),
        }
        Ok(())
    }
}

/// A stretch done: all read, or blank and dropped.
fn retire(atoms: Option<Atoms>, written: bool) {
    let atoms = atoms.expect("a stretch per picture");
    assert!(
        atoms.notes.is_empty() && (!written || atoms.formulas.is_empty()),
        "every formula and note is read"
    );
}

/// GitHub's color for an alert kind.
fn hue(kind: AlertKind) -> &'static str {
    match kind {
        AlertKind::Note => "#0969da",
        AlertKind::Tip => "#1a7f37",
        AlertKind::Important => "#8250df",
        AlertKind::Warning => "#9a6700",
        AlertKind::Caution => "#d1242f",
    }
}

/// The tag a read style writes.
fn tag(style: Style) -> &'static str {
    match style {
        Style::Strong => "strong",
        Style::Emphasis => "em",
        Style::Strike => "del",
        Style::Code => "code",
        Style::Underline => "u",
        Style::Mark => "mark",
        Style::Link | Style::Math | Style::Footnote => unreachable!("{style:?} writes no tag"),
    }
}

/// Line starts; CRLF, LF or a lone CR ends one.
fn line_starts(source: &str) -> Vec<usize> {
    let bytes = source.as_bytes();
    let ends = (bytes.iter().enumerate())
        .filter(|&(at, &byte)| byte == b'\n' || byte == b'\r' && bytes.get(at + 1) != Some(&b'\n'));
    std::iter::once(0).chain(ends.map(|(at, _)| at + 1)).collect()
}

/// An SVG without its XML declaration.
fn bare(svg: &str) -> &str {
    svg.find("<svg").map_or(svg, |at| &svg[at..])
}
