//! The bundled templates (DOC-01).

use ely_platform::settings::Paper;
use ely_gpui_component::i18n::I18n;
use gpui::App;

/// As Chinese Word: compressed punctuation, two-character tabs.
const CHINESE: &str = "<office:settings><config:config-item-set config:name=\"ooo:configuration-settings\"><config:config-item config:name=\"CharacterCompressionType\" config:type=\"short\">1</config:config-item></config:config-item-set></office:settings><office:styles><style:default-style style:family=\"paragraph\"><style:paragraph-properties style:tab-stop-distance=\"21pt\"/></style:default-style></office:styles>";

/// The three bundled templates (DOC-01).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Template {
    Blank,
    Report,
    Letter,
}

pub const TEMPLATES: [Template; 3] = [Template::Blank, Template::Report, Template::Letter];

impl Template {
    pub fn key(self) -> &'static str {
        match self {
            Template::Blank => "blank",
            Template::Report => "report",
            Template::Letter => "letter",
        }
    }

    /// The paragraphs, as (heading level, text); 0 is body.
    fn lines(self, cx: &App) -> Vec<(u8, String)> {
        let text = |key: &str| ely_ui::i18n::text(cx, key, &[]).to_string();
        match self {
            Template::Blank => vec![(0, String::new())],
            Template::Report => vec![
                (9, text("doc.report.title")),
                (1, text("doc.report.summary")),
                (0, text("doc.body")),
                (1, text("doc.report.details")),
                (0, text("doc.body")),
            ],
            Template::Letter => vec![
                (0, text("doc.letter.sender")),
                (0, text("doc.letter.date")),
                (0, String::new()),
                (0, text("doc.letter.recipient")),
                (0, String::new()),
                (0, text("doc.letter.greeting")),
                (0, text("doc.body")),
                (0, String::new()),
                (0, text("doc.letter.closing")),
                (0, text("doc.letter.sender")),
            ],
        }
    }

    /// Flat ODF text; level 9 is the Title style.
    pub fn fodt(self, cx: &App) -> String {
        let body: String = self
            .lines(cx)
            .into_iter()
            .map(|(level, text)| {
                let text = escape(&text);
                match level {
                    0 => format!("<text:p>{text}</text:p>"),
                    9 => format!("<text:p text:style-name=\"Title\">{text}</text:p>"),
                    level => format!("<text:h text:outline-level=\"{level}\">{text}</text:h>"),
                }
            })
            .collect();
        let (width, height) = page(ely_ui::settings::current(cx).paper);
        let chinese = cx.global::<I18n>().locale().tag == "zh-CN";
        let defaults = if chinese { CHINESE } else { "" };
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<office:document xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:style=\"urn:oasis:names:tc:opendocument:xmlns:style:1.0\" xmlns:fo=\"urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0\" xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" xmlns:config=\"urn:oasis:names:tc:opendocument:xmlns:config:1.0\" xmlns:ooo=\"http://openoffice.org/2004/office\" office:version=\"1.3\" office:mimetype=\"application/vnd.oasis.opendocument.text\">{defaults}<office:automatic-styles><style:page-layout style:name=\"pm1\"><style:page-layout-properties fo:page-width=\"{width}\" fo:page-height=\"{height}\" style:print-orientation=\"portrait\" fo:margin-top=\"2cm\" fo:margin-bottom=\"2cm\" fo:margin-left=\"2cm\" fo:margin-right=\"2cm\"/></style:page-layout></office:automatic-styles><office:master-styles><style:master-page style:name=\"Standard\" style:page-layout-name=\"pm1\"/></office:master-styles><office:body><office:text>{body}</office:text></office:body></office:document>\n"
        )
    }

    /// The template as the picker's markdown preview.
    pub fn preview(self, cx: &App) -> String {
        self.lines(cx)
            .into_iter()
            .map(|(level, text)| match level {
                0 => text,
                9 => format!("# {text}"),
                level => format!("{} {text}", "#".repeat(usize::from(level) + 1)),
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// Width and height; margins stay the kernel's 2 cm.
fn page(paper: Paper) -> (&'static str, &'static str) {
    let region = ely_platform::locale::region();
    let letter = paper.letter(region.as_deref());
    log::info!("doc: new page {paper:?}, region {region:?}, letter {letter}");
    if letter { ("8.5in", "11in") } else { ("21cm", "29.7cm") }
}

/// XML-escapes template text.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_text_is_escaped() {
        assert_eq!(escape("a<b & c>"), "a&lt;b &amp; c&gt;");
    }
}
