//! Images in pasted HTML, checked before the engine reads them.

use std::io;

use super::input::invalid;

/// Each `img` tag's `src`, entities decoded.
#[derive(Default)]
struct Sources(std::cell::RefCell<Vec<String>>);

impl html5ever::tokenizer::TokenSink for Sources {
    type Handle = ();

    fn process_token(&self, token: html5ever::tokenizer::Token, _: u64) -> html5ever::tokenizer::TokenSinkResult<()> {
        use html5ever::tokenizer::{TagKind, Token};
        if let Token::TagToken(tag) = token
            && tag.kind == TagKind::StartTag
            && &*tag.name == "img"
            && let Some(src) = tag.attrs.iter().find(|attr| &*attr.name.local == "src")
        {
            self.0.borrow_mut().push(src.value.to_string());
        }
        html5ever::tokenizer::TokenSinkResult::Continue
    }
}

/// Every image a `data:` URI carries must decode.
pub(super) fn check_html_images(html: &str) -> io::Result<()> {
    use base64::Engine;
    use html5ever::tokenizer::{BufferQueue, Tokenizer, TokenizerOpts};
    let tokenizer = Tokenizer::new(Sources::default(), TokenizerOpts::default());
    let input = BufferQueue::default();
    input.push_back(html5ever::tendril::StrTendril::from_slice(html));
    let _ = tokenizer.feed(&input);
    tokenizer.end();
    for src in tokenizer.sink.0.take() {
        let src = src.trim_matches(|c: char| c.is_ascii_whitespace() || c.is_ascii_control());
        let Some(uri) = src.get(..5).filter(|scheme| scheme.eq_ignore_ascii_case("data:")).map(|_| &src[5..]) else {
            continue;
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
    }
    Ok(())
}
