//! Markdown as plain text: words, code and TeX, no marks.

use std::io;

use ely_gpui_component::documents::{
    MarkdownBlock, MarkdownReader, alert, is_toc, markdown_tree, mdast::Node,
};

/// `text` as plain text, a blank line between blocks.
pub fn plain(text: &str) -> io::Result<String> {
    let tree = markdown_tree(text);
    let reader = MarkdownReader::joined(text, &tree);
    let mut blocks = Vec::new();
    // Alerts and `[toc]` live only at the root.
    for child in tree.children().into_iter().flatten() {
        match child {
            Node::Blockquote(quote) => match alert(quote) {
                Some((kind, body)) => {
                    blocks.push(kind.title().to_string());
                    for node in &body {
                        block(&reader, node, &mut blocks);
                    }
                }
                None => block(&reader, child, &mut blocks),
            },
            _ if is_toc(child, text) => {
                let titles: Vec<&str> = (reader.headings.iter())
                    .map(|heading| heading.title.as_str())
                    .collect();
                if !titles.is_empty() {
                    blocks.push(titles.join("\n"));
                }
            }
            _ => block(&reader, child, &mut blocks),
        }
    }
    Ok(blocks.join("\n\n") + "\n")
}

fn block(reader: &MarkdownReader, node: &Node, blocks: &mut Vec<String>) {
    match node {
        Node::Code(code) => blocks.push(code.value.clone()),
        Node::Math(math) => blocks.push(math.value.clone()),
        Node::List(list) => {
            let items = list.children.iter().map(|item| {
                let mut inner = Vec::new();
                for child in item.children().into_iter().flatten() {
                    block(reader, child, &mut inner);
                }
                inner.join("\n")
            });
            blocks.push(items.collect::<Vec<_>>().join("\n"));
        }
        Node::Table(table) => {
            let rows = table.children.iter().map(|row| {
                // Cells past the head stay undrawn.
                let cells = row.children().into_iter().flatten().take(table.align.len());
                let cells = cells.map(|cell| words(reader, cell));
                cells.collect::<Vec<_>>().join("\t")
            });
            blocks.push(rows.collect::<Vec<_>>().join("\n"));
        }
        Node::FootnoteDefinition(note) => {
            let mut body = Vec::new();
            for child in &note.children {
                block(reader, child, &mut body);
            }
            // The label the drawing shows beside it.
            let label = format!("[{}]", reader.label(note));
            blocks.push([label, body.join("\n\n")].join(" ").trim_end().to_string());
        }
        Node::Blockquote(_) => {
            for child in node.children().into_iter().flatten() {
                block(reader, child, blocks);
            }
        }
        Node::Paragraph(_) => {
            let mut read = Vec::new();
            reader.block(node, &mut read);
            match read.as_slice() {
                [MarkdownBlock::Math(tex)] => blocks.push(tex.clone()),
                _ => blocks.push(words(reader, node)),
            }
        }
        Node::Heading(_) => blocks.push(words(reader, node)),
        Node::Html(_) => {
            let mut read = Vec::new();
            reader.block(node, &mut read);
            for piece in read {
                match piece {
                    MarkdownBlock::Paragraph(text) => blocks.push(text.text),
                    other => unreachable!("an HTML block reads as text, not {other:?}"),
                }
            }
        }
        // Front matter, rules and definitions are not words.
        _ => {}
    }
}

/// An inline run's words, as the drawing reads them.
fn words(reader: &MarkdownReader, node: &Node) -> String {
    reader
        .inline(node.children().expect("an inline run holds inlines"))
        .text
}
