//! Calc's text-import records and fields (sc impex.cxx).

use ely_lok::sheet::csv::Import;

/// Rows as the kernel reads them: no merge or trim.
pub(super) fn rows<'a>(text: &'a str, how: &'a Import) -> impl Iterator<Item = Vec<String>> + 'a {
    let lines = lines(text);
    let mut next = 0;
    std::iter::from_fn(move || {
        let mut record: Vec<char> = lines.get(next)?.chars().collect();
        next += 1;
        if let Some(quote) = how.quote {
            next = join(&lines, next, &mut record, quote, how.separator);
        }
        Some(fields(&record, how.quote, how.separator))
    })
}

/// SvStream lines: CR, LF, CRLF or LFCR ends one.
fn lines(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let (mut found, mut start, mut at) = (Vec::new(), 0, 0);
    while at < bytes.len() {
        let byte = bytes[at];
        at += 1;
        if byte == b'\n' || byte == b'\r' {
            found.push(&text[start..at - 1]);
            if bytes.get(at).is_some_and(|next| matches!(next, b'\n' | b'\r') && *next != byte) {
                at += 1;
            }
            start = at;
        }
    }
    if start < bytes.len() {
        found.push(&text[start..]);
    }
    found
}

/// A quote's part in an open field (QuoteType).
#[derive(Clone, Copy, PartialEq)]
enum Quote {
    Open,
    End,
    Unknown,
}

/// The quote at `at` ends its field, blanks allowed.
fn ends_field(record: &[char], mut at: usize, separator: char) -> bool {
    if separator == ' ' && record.get(at + 1) == Some(&' ') {
        return true;
    }
    while record.get(at + 1) == Some(&' ') {
        at += 1;
    }
    record.get(at + 1).is_none_or(|next| *next == separator)
}

/// After a blank or separator a field may start.
fn starts(start: bool, c: char, separator: char) -> bool {
    c == separator || (start && c == ' ')
}

/// ReadCsvLine: lines join while a quoted field stays open.
fn join(lines: &[&str], mut next: usize, record: &mut Vec<char>, quote: char, separator: char) -> usize {
    let (mut kept, mut back) = (record.len(), next);
    let (mut quotes, mut state, mut start, mut from) = (0u32, Quote::End, true, 0);
    loop {
        for at in from..record.len() {
            let c = record[at];
            if c == quote && start {
                quotes += 1;
                (state, start, kept, back) = (Quote::Open, false, record.len(), next);
            } else if c == quote && quotes > 0 && state != Quote::End {
                state = match () {
                    _ if quotes % 2 == 0 || record.get(at + 1) == Some(&quote) => Quote::Open,
                    _ if ends_field(record, at, separator) => Quote::End,
                    _ => Quote::Unknown,
                };
                quotes += u32::from(state != Quote::Unknown);
            } else if c != quote && state == Quote::End {
                start = starts(start, c, separator);
            }
        }
        if quotes % 2 == 0 || state == Quote::Unknown || next >= lines.len() {
            break;
        }
        from = record.len();
        record.push('\n');
        record.extend(lines[next].chars());
        next += 1;
    }
    // Unclosed: stop at the line it opened on.
    if quotes % 2 == 1 {
        record.truncate(kept);
        return back;
    }
    next
}

/// ExtText2Doc's fields; an empty record is one empty field.
fn fields(record: &[char], quote: Option<char>, separator: char) -> Vec<String> {
    let mut row = Vec::new();
    let mut at = 0;
    while at < record.len() || row.is_empty() {
        let (mut value, next) = field(record, at, quote, separator);
        // File imports drop one trailing line feed.
        if value.ends_with('\n') {
            value.pop();
        }
        row.push(value);
        at = next;
    }
    row
}

/// ScanNextFieldFromString: one field and where the next starts.
fn field(record: &[char], mut at: usize, quote: Option<char>, separator: char) -> (String, usize) {
    let mut value = String::new();
    if let Some(quote) = quote {
        // Blanks before a quote go unless blank separates.
        let blanks = record[at..].iter().take_while(|c| **c == ' ').count();
        if separator != ' ' && record.get(at + blanks) == Some(&quote) {
            at += blanks;
        }
        if record.get(at) == Some(&quote) {
            at = quoted(record, at, quote, separator, &mut value);
        }
    }
    let stop = at + record[at..].iter().take_while(|c| **c != separator).count();
    value.extend(&record[at..stop]);
    (value, stop + usize::from(stop < record.len()))
}

/// lcl_ScanString: past the closing quote, or `start` if none.
fn quoted(record: &[char], start: usize, quote: char, separator: char, value: &mut String) -> usize {
    let (mut text, mut from, mut at) = (String::new(), start + 1, start + 1);
    loop {
        let Some(&c) = record.get(at) else {
            return start;
        };
        if c != quote {
            at += 1;
        } else if record.get(at + 1) == Some(&quote) {
            text.extend(&record[from..=at]);
            (at, from) = (at + 2, at + 2);
        } else if ends_field(record, at, separator) {
            text.extend(&record[from..at]);
            value.push_str(&text);
            return at + 1;
        } else {
            at += 1;
        }
    }
}
