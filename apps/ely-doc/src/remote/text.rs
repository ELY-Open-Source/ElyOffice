//! Text sent to Writer: replacements with breaks, appended lines.

/// Why a FIND is refused.
pub(super) const FIND_PARAGRAPH: &str = "FIND must be non-empty text within one paragraph";
/// Why WITH with breaks cannot be written as asked.
pub(super) const WITH_BACKSLASH: &str = "WITH with newlines cannot contain literal \\n or \\ before a newline; use two replaces";
/// Why an append is refused.
pub(super) const APPEND_EMPTY: &str = "TEXT must not be empty";
/// A `--style` the document lacks.
pub(super) const STYLE_MISSING: &str = "{style}: no such paragraph style in this document";

/// Why `style` cannot be used; None when it can.
pub(super) fn style_missing(worker: &mut ely_lok::Worker, doc: &ely_lok::Doc, style: &str) -> std::io::Result<Option<String>> {
    if style.is_empty() {
        return Ok(None);
    }
    let listed = ely_lok::commands::styles(&worker.values(doc, ".uno:StyleApply")?, "2")?;
    Ok((!listed.iter().any(|one| one == style)).then(|| STYLE_MISSING.replace("{style}", style)))
}

/// Line breaks as `\n`, whatever the sender's system.
fn unix(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// FIND and WITH as a regex; None without a break.
pub(super) fn breaks(find: &str, with: &str) -> Result<Option<(String, String)>, &'static str> {
    let (find, with) = (unix(find), unix(with));
    if find.is_empty() || find.contains('\n') {
        return Err(FIND_PARAGRAPH);
    }
    if !with.contains('\n') {
        return Ok(None);
    }
    // Writer reads these as breaks, whatever the escaping.
    if with.contains("\\n") || with.contains("\\\n") {
        return Err(WITH_BACKSLASH);
    }
    let quoted = format!("\\Q{}\\E", find.replace("\\E", "\\E\\\\E\\Q"));
    let mut escaped = String::with_capacity(with.len() * 2);
    for char in with.chars() {
        match char {
            '\\' => escaped.push_str("\\\\"),
            '&' => escaped.push_str("\\&"),
            '$' => escaped.push_str("\\$"),
            '\n' => escaped.push_str("\\n"),
            other => escaped.push(other),
        }
    }
    Ok(Some((quoted, escaped)))
}

/// TEXT's lines to append, less one closing break.
pub(super) fn lines(text: &str) -> Result<String, &'static str> {
    let text = unix(text);
    let text = text.strip_suffix('\n').unwrap_or(&text);
    if text.is_empty() {
        return Err(APPEND_EMPTY);
    }
    Ok(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_a_break_the_plain_replace_stays() {
        assert_eq!(breaks("cat", "dog"), Ok(None));
        assert_eq!(breaks("cat", "a \\n & $1 b"), Ok(None), "no break: nothing to escape");
        assert_eq!(breaks("", "x"), Err(FIND_PARAGRAPH));
        assert_eq!(breaks("a\nb", "x"), Err(FIND_PARAGRAPH));
        assert_eq!(breaks("a\r\nb", "x"), Err(FIND_PARAGRAPH), "a Windows break too");
    }

    #[test]
    fn a_break_quotes_find_and_escapes_with() {
        let (find, with) = breaks("a.b (c)$", "one & $1\\ two\r\nthree").expect("fine").expect("breaks");
        assert_eq!(find, "\\Qa.b (c)$\\E");
        assert_eq!(with, "one \\& \\$1\\\\ two\\nthree");
        let (find, _) = breaks("x\\Ey", "a\nb").expect("fine").expect("breaks");
        assert_eq!(find, "\\Qx\\E\\\\E\\Qy\\E", "a \\E in FIND stays literal");
    }

    #[test]
    fn a_backslash_writer_would_read_as_a_break_is_refused() {
        assert_eq!(breaks("x", "a\\nb\nc"), Err(WITH_BACKSLASH));
        assert_eq!(breaks("x", "a\\\nb"), Err(WITH_BACKSLASH));
        assert!(breaks("x", "a\\tb\nc").is_ok(), "other backslashes are fine");
    }

    #[test]
    fn appended_lines_drop_one_closing_break() {
        assert_eq!(lines("a\r\nb\n"), Ok("a\nb".into()));
        assert_eq!(lines("a\n\n"), Ok("a\n".into()), "a blank last line stays");
        assert_eq!(lines(" "), Ok(" ".into()), "spaces are a paragraph");
        assert_eq!(lines(""), Err(APPEND_EMPTY));
        assert_eq!(lines("\n"), Err(APPEND_EMPTY));
    }
}
