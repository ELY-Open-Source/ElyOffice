//! Images in pasted HTML, checked before the engine reads them.

use std::cell::RefCell;
use std::io;

use html5ever::tokenizer::{BufferQueue, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts};

use super::input::invalid;

/// Every `src` and `data` value, and each comment's text.
#[derive(Default)]
struct Found {
    values: RefCell<Vec<String>>,
    comments: RefCell<Vec<String>>,
}

impl TokenSink for Found {
    type Handle = ();

    fn process_token(&self, token: Token, _: u64) -> TokenSinkResult<()> {
        match token {
            // The engine's own parser may see tags elsewhere.
            Token::TagToken(tag) => self.values.borrow_mut().extend(
                tag.attrs
                    .iter()
                    .filter(|attr| matches!(&*attr.name.local, "src" | "data"))
                    .map(|attr| attr.value.to_string()),
            ),
            Token::CommentToken(text) => self.comments.borrow_mut().push(text.to_string()),
            _ => {}
        }
        TokenSinkResult::Continue
    }
}

/// Every image a `data:` URI carries must decode.
pub(super) fn check_html_images(html: &str) -> io::Result<()> {
    let mut pending = vec![html.to_string()];
    while let Some(html) = pending.pop() {
        let tokenizer = Tokenizer::new(Found::default(), TokenizerOpts::default());
        let input = BufferQueue::default();
        input.push_back(html5ever::tendril::StrTendril::from_slice(&html));
        let _ = tokenizer.feed(&input);
        tokenizer.end();
        pending.extend(tokenizer.sink.comments.take());
        for value in tokenizer.sink.values.take() {
            check_value(&value)?;
        }
    }
    Ok(())
}

fn check_value(value: &str) -> io::Result<()> {
    use base64::Engine;
    // The engine drops CR, LF and escaping backslashes.
    let kept: String = value.chars().filter(|c| !matches!(c, '\r' | '\n' | '\\')).collect();
    let bare: String = kept.chars().filter(|c| !c.is_ascii_whitespace() && !c.is_ascii_control()).collect();
    if !bare.get(..5).is_some_and(|scheme| scheme.eq_ignore_ascii_case("data:")) {
        return Ok(());
    }
    let kept = kept.trim_matches(|c: char| c.is_ascii_whitespace() || c.is_ascii_control());
    let Some(uri) = kept.get(..5).filter(|scheme| scheme.eq_ignore_ascii_case("data:")).map(|_| &kept[5..]) else {
        return Err(invalid("an inline image breaks its scheme"));
    };
    let (meta, body) = uri.split_once(',').ok_or_else(|| invalid("an inline image has no data"))?;
    let base64 = meta.len().checked_sub(7).and_then(|at| meta.get(at..)).is_some_and(|end| end.eq_ignore_ascii_case(";base64"));
    if !base64 {
        return Err(invalid("an inline image is not base64"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(body)
        .map_err(|error| invalid(&format!("an inline image is bad base64: {error}")))?;
    image::load_from_memory(&bytes)
        .map_err(|error| invalid(&format!("an inline image does not decode: {error}")))?;
    Ok(())
}
