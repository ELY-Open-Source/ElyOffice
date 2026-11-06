//! A letter's Markdown as HTML, as composed.

use std::collections::HashMap;

use ely_gpui_component::documents::{
    AlignKind, markdown_tree,
    mdast::{FootnoteDefinition, Node},
};
use ely_mail_engine::page::link;

/// `words` as HTML; raw HTML shows as text.
pub fn html(words: &str) -> String {
    let tree = markdown_tree(words);
    let mut writer = Writer {
        source: words,
        urls: HashMap::new(),
        notes: HashMap::new(),
        cited: Vec::new(),
        mark: None,
        out: String::new(),
    };
    writer.gather(&tree);
    writer.block(&tree, false);
    writer.footnotes();
    writer.out
}

struct Writer<'a> {
    source: &'a str,
    /// Link definitions by label.
    urls: HashMap<&'a str, &'a str>,
    notes: HashMap<&'a str, &'a FootnoteDefinition>,
    /// Footnotes in the order first cited.
    cited: Vec<&'a str>,
    /// A task box for the next paragraph.
    mark: Option<&'static str>,
    out: String,
}

impl<'a> Writer<'a> {
    fn gather(&mut self, node: &'a Node) {
        match node {
            Node::Definition(it) => {
                self.urls.entry(&it.identifier).or_insert(&it.url);
            }
            Node::FootnoteDefinition(it) => {
                self.notes.entry(&it.identifier).or_insert(it);
            }
            _ => {}
        }
        for child in node.children().into_iter().flatten() {
            self.gather(child);
        }
    }

    fn push(&mut self, text: &str) {
        self.out.push_str(text);
    }

    /// The source `node` spans, as text.
    fn written(&mut self, node: &Node) {
        let text = escape(&self.source[node.position().clone()]);
        self.push(&text);
    }

    fn blocks(&mut self, nodes: &'a [Node], tight: bool) {
        for node in nodes {
            self.block(node, tight);
        }
    }

    fn inlines(&mut self, nodes: &'a [Node]) {
        for node in nodes {
            self.inline(node);
        }
    }

    /// `tight`: a tight list's paragraph, without `<p>`.
    fn block(&mut self, node: &'a Node, tight: bool) {
        match node {
            Node::Root(it) => self.blocks(&it.children, false),
            Node::Blockquote(it) => {
                self.push("<blockquote>\n");
                self.blocks(&it.children, false);
                self.push("</blockquote>\n");
            }
            Node::Paragraph(it) => {
                if !tight {
                    self.push("<p>");
                }
                if let Some(mark) = self.mark.take() {
                    self.push(mark);
                }
                self.inlines(&it.children);
                if !tight {
                    self.push("</p>\n");
                }
            }
            Node::Heading(it) => self.wrapped(&format!("h{}", it.depth), &it.children),
            Node::ThematicBreak(_) => self.push("<hr>\n"),
            Node::List(list) => {
                let tag = if list.ordered { "ol" } else { "ul" };
                match list.start.filter(|start| list.ordered && *start != 1) {
                    Some(start) => self.push(&format!("<ol start=\"{start}\">\n")),
                    None => self.push(&format!("<{tag}>\n")),
                }
                self.blocks(&list.children, !list.spread);
                self.push(&format!("</{tag}>\n"));
            }
            Node::ListItem(item) => {
                self.push("<li>");
                let mark = item.checked.map(|done| if done { "☑ " } else { "☐ " });
                // The box sits on its text's line.
                match item.children.first() {
                    Some(Node::Paragraph(_)) => self.mark = mark,
                    _ => self.push(mark.unwrap_or_default()),
                }
                self.blocks(&item.children, tight);
                self.push("</li>\n");
            }
            Node::Code(code) => {
                match &code.lang {
                    Some(lang) => self.push(&format!("<pre><code class=\"language-{}\">", escape(lang))),
                    None => self.push("<pre><code>"),
                }
                self.push(&escape(&code.value));
                self.push("</code></pre>\n");
            }
            // Mail shows no TeX or front matter.
            Node::Math(_) | Node::Yaml(_) | Node::Toml(_) => {
                self.push("<pre>");
                self.written(node);
                self.push("</pre>\n");
            }
            Node::Html(_) => {
                self.push("<p>");
                self.written(node);
                self.push("</p>\n");
            }
            Node::Table(table) => self.table(&table.children, &table.align),
            Node::TableRow(it) | Node::TableCell(it) => self.inlines(&it.children),
            Node::Definition(_) | Node::FootnoteDefinition(_) => {}
            _ => self.inline(node),
        }
    }

    fn wrapped(&mut self, tag: &str, children: &'a [Node]) {
        self.push(&format!("<{tag}>"));
        self.inlines(children);
        self.push(&format!("</{tag}>\n"));
    }

    /// Every row has the header's columns, as GFM reads it.
    fn table(&mut self, rows: &'a [Node], align: &[AlignKind]) {
        self.push("<table>\n");
        for (at, row) in rows.iter().enumerate() {
            let cell = if at == 0 { "th" } else { "td" };
            let cells = row.children().map_or(&[][..], Vec::as_slice);
            self.push("<tr>");
            for (column, side) in align.iter().enumerate() {
                let side = match side {
                    AlignKind::Left => " style=\"text-align:left\"",
                    AlignKind::Right => " style=\"text-align:right\"",
                    AlignKind::Center => " style=\"text-align:center\"",
                    AlignKind::None => "",
                };
                self.push(&format!("<{cell}{side}>"));
                if let Some(node) = cells.get(column) {
                    self.block(node, true);
                }
                self.push(&format!("</{cell}>"));
            }
            self.push("</tr>\n");
        }
        self.push("</table>\n");
    }

    fn inline(&mut self, node: &'a Node) {
        match node {
            Node::Text(it) => self.push(&escape(&it.value)),
            Node::Strong(it) => self.styled("strong", &it.children),
            Node::Emphasis(it) => self.styled("em", &it.children),
            Node::Delete(it) => self.styled("del", &it.children),
            Node::InlineCode(it) => self.push(&format!("<code>{}</code>", escape(&it.value))),
            Node::InlineMath(_) | Node::Html(_) => self.written(node),
            Node::Break(_) => self.push("<br>\n"),
            Node::Link(it) => self.anchor(&it.url, &it.children),
            Node::LinkReference(it) => match self.urls.get(it.identifier.as_str()) {
                Some(url) => self.anchor(url, &it.children),
                None => self.written(node),
            },
            Node::Image(it) => self.picture(&it.url, &it.alt),
            Node::ImageReference(it) => match self.urls.get(it.identifier.as_str()) {
                Some(url) => self.picture(url, &it.alt),
                None => self.written(node),
            },
            Node::FootnoteReference(it) => match self.notes.get_key_value(it.identifier.as_str()) {
                Some((id, _)) => {
                    let number = match self.cited.iter().position(|cited| cited == id) {
                        Some(at) => at + 1,
                        None => {
                            self.cited.push(*id);
                            self.cited.len()
                        }
                    };
                    self.push(&format!("<sup>{number}</sup>"));
                }
                None => self.written(node),
            },
            _ => self.block(node, true),
        }
    }

    fn styled(&mut self, tag: &str, children: &'a [Node]) {
        self.push(&format!("<{tag}>"));
        self.inlines(children);
        self.push(&format!("</{tag}>"));
    }

    /// An unsafe address keeps its text, not its link.
    fn anchor(&mut self, url: &str, children: &'a [Node]) {
        let Some(url) = link(url) else {
            return self.inlines(children);
        };
        self.push(&format!("<a href=\"{}\">", escape(&url)));
        self.inlines(children);
        self.push("</a>");
    }

    fn picture(&mut self, url: &str, alt: &str) {
        match link(url) {
            Some(url) => self.push(&format!("<img src=\"{}\" alt=\"{}\">", escape(&url), escape(alt))),
            None => self.push(&escape(alt)),
        }
    }

    /// Notes, numbered as cited, after the letter.
    fn footnotes(&mut self) {
        if self.cited.is_empty() {
            return;
        }
        self.push("<hr>\n<ol>\n");
        // A note may cite one not yet listed.
        let mut at = 0;
        while let Some(id) = self.cited.get(at) {
            let note = self.notes[id];
            at += 1;
            self.push("<li>");
            self.blocks(&note.children, false);
            self.push("</li>\n");
        }
        self.push("</ol>\n");
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
