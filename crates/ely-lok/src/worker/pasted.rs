//! Pasted HTML, rebuilt so the engine reads only checked images.

use std::cell::RefCell;
use std::io;

use base64::Engine;
use html5ever::tokenizer::states::RawKind;
use html5ever::tokenizer::{BufferQueue, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts};

use super::input::invalid;

/// The rebuilt HTML, or the first refusal.
#[derive(Default)]
struct Rebuilt {
    out: RefCell<String>,
    error: RefCell<Option<io::Error>>,
    /// The open raw element and its text so far.
    raw: RefCell<Option<(String, String)>>,
}

impl Rebuilt {
    fn refuse(&self, outcome: io::Result<()>) {
        if let Err(error) = outcome {
            self.error.borrow_mut().get_or_insert(error);
        }
    }

    /// Raw text without `<` ends where both parsers end it.
    fn close_raw(&self) {
        let Some((name, text)) = self.raw.take() else {
            return;
        };
        let markup = match name.as_str() {
            "style" => text.replace("<!--", "").contains('<'),
            _ => text.contains('<'),
        };
        if markup {
            return self.refuse(Err(invalid(&format!("a {name} element holds markup"))));
        }
        if name == "style" {
            self.refuse(check_css(&text));
        }
    }

    fn start(&self, name: &str, attrs: &[html5ever::Attribute]) -> TokenSinkResult<()> {
        let mut out = self.out.borrow_mut();
        out.push('<');
        out.push_str(name);
        for attr in attrs.iter().filter(|attr| plain_name(&attr.name.local)) {
            let key = &*attr.name.local;
            let value = match attribute(name, key, &attr.value) {
                Ok(value) => value,
                Err(error) => {
                    self.refuse(Err(error));
                    continue;
                }
            };
            out.push_str(&format!(" {key}=\"{}\"", escape(&value, true)));
        }
        out.push('>');
        // Only what the engine also reads raw.
        let kind = match name {
            "style" | "xmp" | "noembed" => RawKind::Rawtext,
            "script" => RawKind::ScriptData,
            _ => return TokenSinkResult::Continue,
        };
        self.raw.replace(Some((name.to_owned(), String::new())));
        TokenSinkResult::RawData(kind)
    }
}

impl TokenSink for Rebuilt {
    type Handle = ();

    fn process_token(&self, token: Token, _: u64) -> TokenSinkResult<()> {
        match token {
            Token::DoctypeToken(_) => self.out.borrow_mut().push_str("<!DOCTYPE html>"),
            Token::TagToken(tag) if !plain_name(&tag.name) => {}
            Token::TagToken(tag) if tag.kind == TagKind::StartTag => return self.start(&tag.name, &tag.attrs),
            Token::TagToken(tag) => {
                self.close_raw();
                self.out.borrow_mut().push_str(&format!("</{}>", &*tag.name));
            }
            Token::CharacterTokens(text) => match &mut *self.raw.borrow_mut() {
                Some((_, raw)) => {
                    raw.push_str(&text);
                    self.out.borrow_mut().push_str(&text);
                }
                None => self.out.borrow_mut().push_str(&escape(&text, false)),
            },
            Token::EOFToken => self.close_raw(),
            // The engine's parser would read comments its own way.
            Token::CommentToken(_) | Token::NullCharacterToken | Token::ParseError(_) => {}
        }
        TokenSinkResult::Continue
    }
}

/// Every image the engine could load, decoded first; HTML rewritten.
pub(super) fn rebuild(html: &str) -> io::Result<String> {
    let tokenizer = Tokenizer::new(Rebuilt::default(), TokenizerOpts::default());
    let input = BufferQueue::default();
    input.push_back(html5ever::tendril::StrTendril::from_slice(html));
    let _ = tokenizer.feed(&input);
    tokenizer.end();
    if let Some(error) = tokenizer.sink.error.take() {
        return Err(error);
    }
    Ok(tokenizer.sink.out.take())
}

/// Names the engine reads one way only.
fn plain_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, ':' | '_' | '.' | '-'))
}

fn escape(text: &str, quoted: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if quoted => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

/// A value the engine reads as `data:` becomes canonical.
fn attribute(tag: &str, key: &str, value: &str) -> io::Result<String> {
    if key == "style" {
        check_css(value)?;
    }
    // The engine drops spaces, line breaks and escaping backslashes.
    let bare: String = value.chars().filter(|c| !c.is_whitespace() && !c.is_control() && *c != '\\').collect();
    let Some(uri) = data_uri(&bare) else {
        return Ok(value.to_owned());
    };
    // A link's target is never loaded.
    if key == "href" && matches!(tag, "a" | "area") {
        return Ok(value.to_owned());
    }
    image_uri(uri)
}

fn data_uri(text: &str) -> Option<&str> {
    text.get(..5).filter(|scheme| scheme.eq_ignore_ascii_case("data:")).map(|_| &text[5..])
}

/// `uri` after `data:` as a decoded image, written plainly.
fn image_uri(uri: &str) -> io::Result<String> {
    let (meta, body) = uri.split_once(',').ok_or_else(|| invalid("an inline image has no data"))?;
    let base64 = meta.len().checked_sub(7).and_then(|at| meta.get(at..)).is_some_and(|end| end.eq_ignore_ascii_case(";base64"));
    if !base64 {
        return Err(invalid("an inline image is not base64"));
    }
    let engine = base64::engine::general_purpose::STANDARD;
    let bytes = engine.decode(body).map_err(|error| invalid(&format!("an inline image is bad base64: {error}")))?;
    let format = image::guess_format(&bytes).map_err(|error| invalid(&format!("an inline image has no format: {error}")))?;
    image::load_from_memory_with_format(&bytes, format)
        .map_err(|error| invalid(&format!("an inline image does not decode: {error}")))?;
    Ok(format!("data:{};base64,{}", format.to_mime_type(), engine.encode(&bytes)))
}

/// Images in style text must decode; escaped ones are refused.
fn check_css(css: &str) -> io::Result<()> {
    if css.contains('\\') && unescape_css(css).to_ascii_lowercase().contains("data:") {
        return Err(invalid("an image in a style is escaped"));
    }
    let lower = css.to_ascii_lowercase();
    for (at, _) in lower.match_indices("data:") {
        let rest = &css[at + 5..];
        let end = rest.find(|c: char| matches!(c, ')' | '"' | '\'') || c.is_whitespace()).unwrap_or(rest.len());
        image_uri(&rest[..end])?;
    }
    Ok(())
}

/// CSS escapes: hex code points, line continuations, quoted characters.
fn unescape_css(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut chars = css.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        let mut hex = String::new();
        while hex.len() < 6 && chars.peek().is_some_and(char::is_ascii_hexdigit) {
            hex.extend(chars.next());
        }
        if hex.is_empty() {
            out.extend(chars.next().filter(|next| *next != '\n'));
            continue;
        }
        chars.next_if(|next| next.is_ascii_whitespace());
        out.push(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32).unwrap_or('\u{FFFD}'));
    }
    out
}

#[cfg(test)]
mod tests;
