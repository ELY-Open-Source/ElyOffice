//! CSV and TSV files: read before opening (TABLE-12, §9.3).

use std::{
    iter::Peekable,
    path::{Path, PathBuf},
    str::Chars,
    sync::Arc,
};

use ely_lok::sheet::csv::{Column, Encoding, Import};
use ely_office::{in_window, prelude::*};
use ely_ui::commands::Open;
use gpui::{AppContext as _, Context, TaskExt as _, Window};

use crate::book::Book;
use crate::dialog::Dialog;

mod dialogs;
pub use dialogs::import_dialog;

/// Extensions the import dialog reads.
const TEXT: [&str; 4] = ["csv", "tsv", "tab", "txt"];
/// Rows the preview shows.
pub const SHOWN: usize = 8;
/// Characters a preview cell keeps.
const CELL: usize = 200;
/// Separators offered, in guessing order.
pub const SEPARATORS: [char; 5] = [',', ';', '\t', '|', ' '];

fn is_text(path: &Path) -> bool {
    let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
    TEXT.contains(&ext.to_ascii_lowercase().as_str())
}

/// A text file waiting for its import choices.
pub struct Pending {
    pub path: PathBuf,
    bytes: Arc<Vec<u8>>,
    pub how: Import,
    /// The first rows as the choices read them.
    pub rows: Vec<Vec<String>>,
    /// The chosen encoding does not read the file.
    pub broken: bool,
    /// A scan of the whole file is on its way.
    pub reading: bool,
    /// The latest scan asked; older ones are dropped.
    asked: u64,
}

/// What one scan of the whole file found.
struct Scan {
    rows: Vec<Vec<String>>,
    columns: Vec<Column>,
    broken: bool,
}

impl Pending {
    pub(crate) fn new(path: PathBuf, bytes: Vec<u8>, legacy: Encoding) -> Self {
        let how = Import {
            separator: guess_separator(&path, &bytes),
            quote: Some('"'),
            encoding: guess_encoding(&bytes, legacy),
            columns: Vec::new(),
        };
        Pending {
            path,
            bytes: Arc::new(bytes),
            how,
            rows: Vec::new(),
            broken: false,
            reading: true,
            asked: 0,
        }
    }
}

/// Reads every row: preview, column kinds, decoding.
fn scan(bytes: &[u8], how: &Import) -> Scan {
    let Some(text) = decode(bytes, how.encoding) else {
        return Scan {
            rows: Vec::new(),
            columns: Vec::new(),
            broken: true,
        };
    };
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    for row in split(&text, how) {
        if columns.len() < row.len() {
            columns.resize(row.len(), Column::Standard);
        }
        for (at, value) in row.iter().enumerate() {
            if is_code(value.trim()) {
                columns[at] = Column::Text;
            }
        }
        if rows.len() < SHOWN {
            rows.push(row.iter().map(|value| value.chars().take(CELL).collect()).collect());
        }
    }
    Scan {
        rows,
        columns,
        broken: false,
    }
}

/// The Windows code page of the app's language.
pub(crate) fn legacy_encoding(locale: &str) -> Encoding {
    match locale {
        "zh-CN" => Encoding::Gb18030,
        "ja-JP" => Encoding::ShiftJis,
        "ko-KR" => Encoding::Windows949,
        "vi-VN" => Encoding::Windows1258,
        _ => Encoding::Windows1252,
    }
}

fn guess_encoding(sample: &[u8], legacy: Encoding) -> Encoding {
    if sample.starts_with(&[0xFF, 0xFE]) || sample.starts_with(&[0xFE, 0xFF]) {
        return Encoding::Utf16;
    }
    match std::str::from_utf8(sample) {
        Ok(_) => Encoding::Utf8,
        Err(_) => legacy,
    }
}

/// TSV by name, else line one's commonest separator.
fn guess_separator(path: &Path, sample: &[u8]) -> char {
    let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
    if ["tsv", "tab"].contains(&ext.to_ascii_lowercase().as_str()) {
        return '\t';
    }
    let line = sample.split(|byte| *byte == b'\n').next().unwrap_or(&[]);
    let count = |separator: char| line.iter().filter(|byte| char::from(**byte) == separator).count();
    SEPARATORS[..4]
        .iter()
        .copied()
        .rev()
        .max_by_key(|separator| count(*separator))
        .filter(|separator| count(*separator) > 0)
        .unwrap_or(',')
}

/// The text, or None where a byte breaks the encoding.
fn decode(bytes: &[u8], encoding: Encoding) -> Option<String> {
    let (codec, bom): (&'static encoding_rs::Encoding, &[u8]) = match encoding {
        Encoding::Utf8 => (encoding_rs::UTF_8, &[0xEF, 0xBB, 0xBF]),
        Encoding::Utf16 if bytes.starts_with(&[0xFE, 0xFF]) => (encoding_rs::UTF_16BE, &[0xFE, 0xFF]),
        Encoding::Utf16 => (encoding_rs::UTF_16LE, &[0xFF, 0xFE]),
        Encoding::Gb18030 => (encoding_rs::GB18030, &[]),
        Encoding::Big5 => (encoding_rs::BIG5, &[]),
        Encoding::ShiftJis => (encoding_rs::SHIFT_JIS, &[]),
        // WHATWG's EUC-KR is Windows-949.
        Encoding::Windows949 => (encoding_rs::EUC_KR, &[]),
        Encoding::Windows1252 => (encoding_rs::WINDOWS_1252, &[]),
        Encoding::Windows1251 => (encoding_rs::WINDOWS_1251, &[]),
        Encoding::Windows1258 => (encoding_rs::WINDOWS_1258, &[]),
        // Latin-1 maps each byte to its code point.
        Encoding::Latin1 => return Some(bytes.iter().map(|byte| char::from(*byte)).collect()),
    };
    let body = bytes.strip_prefix(bom).unwrap_or(bytes);
    codec
        .decode_without_bom_handling_and_without_replacement(body)
        .map(|text| text.into_owned())
}

/// Calc's text-import rows (impex.cxx), one at a time.
fn split<'a>(text: &'a str, how: &'a Import) -> impl Iterator<Item = Vec<String>> + 'a {
    let mut chars = text.chars().peekable();
    std::iter::from_fn(move || {
        chars.peek()?;
        let mut row = Vec::new();
        loop {
            let (value, more) = field(&mut chars, how);
            row.push(value);
            if !more {
                return Some(row);
            }
        }
    })
}

/// One field, and whether the record goes on.
fn field(chars: &mut Peekable<Chars>, how: &Import) -> (String, bool) {
    let mut value = String::new();
    if let Some(quote) = how.quote {
        // Calc skips blanks before a quote unless blank splits.
        let mut ahead = chars.clone();
        while how.separator != ' ' && ahead.next_if_eq(&' ').is_some() {}
        if ahead.peek() == Some(&quote) {
            *chars = ahead;
            chars.next();
            while let Some(c) = chars.next() {
                match c {
                    c if c == quote && chars.next_if_eq(&quote).is_some() => value.push(quote),
                    c if c == quote => break,
                    c => value.push(c),
                }
            }
        }
    }
    while let Some(c) = chars.next() {
        match c {
            '\n' => return (value, false),
            '\r' => {
                chars.next_if_eq(&'\n');
                return (value, false);
            }
            c if c == how.separator => return (value, true),
            c => value.push(c),
        }
    }
    (value, false)
}

/// A plus sign, leading zeros, or over 15 digits.
fn is_code(value: &str) -> bool {
    let digits = value.trim_start_matches(['+', '-']);
    let signs = value.len() - digits.len();
    !digits.is_empty()
        && signs <= 1
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && (value.starts_with('+')
            || (digits.len() > 1 && digits.starts_with('0'))
            || digits.len() > 15)
}

impl Book {
    /// Opens a workbook; text files ask how to read first.
    pub(crate) fn on_open_file(&mut self, _: &Open, _: &mut Window, cx: &mut Context<Self>) {
        self.pick(false, cx, Self::open_asked);
    }

    /// Opens a workbook; a text file asks first.
    pub(crate) fn open_asked(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        match is_text(&path) {
            true => self.ask_import(path, cx),
            false => self.open_file(path, cx),
        }
    }

    pub(crate) fn ask_import(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let reading = path.clone();
        // The language the app shows, not the system's.
        let legacy = legacy_encoding(cx.global::<ely_gpui_component::i18n::I18n>().locale().tag);
        cx.spawn(async move |this, cx| {
            // Guessing checks the whole file: off the window thread.
            let pending = cx
                .background_spawn(async move {
                    std::fs::read(&reading).map(|bytes| Pending::new(reading, bytes, legacy))
                })
                .await;
            in_window(&this, cx, |book, _, cx| match pending {
                Ok(pending) => {
                    log::info!("table: import asks for {} bytes", pending.bytes.len());
                    book.imports.push_back(pending);
                    book.next_import(cx);
                }
                Err(error) => book.failed_open(&path.display().to_string(), &error, cx),
            })
        })
        .detach_and_log_err(cx);
    }

    /// Opens `dialog`; a displaced import waits its turn again.
    pub(crate) fn open_dialog(&mut self, dialog: Dialog) {
        if let Some(Dialog::Import(pending)) = self.dialog.replace(dialog) {
            log::info!("table: import waits behind another dialog");
            self.imports.push_front(*pending);
        }
    }

    /// The next waiting import asks, once no dialog is open.
    pub(crate) fn next_import(&mut self, cx: &mut Context<Self>) {
        if self.dialog.is_some() {
            return;
        }
        if let Some(pending) = self.imports.pop_front() {
            // A finished scan keeps the reader's column choices.
            let unread = pending.reading;
            self.dialog = Some(Dialog::Import(Box::new(pending)));
            if unread {
                self.scan_import(cx);
            }
        }
    }

    /// Reads the whole file again for the dialog's choices.
    pub(crate) fn scan_import(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Import(pending)) = &mut self.dialog else {
            return;
        };
        pending.asked += 1;
        pending.reading = true;
        let (bytes, how, asked) = (Arc::clone(&pending.bytes), pending.how.clone(), pending.asked);
        let scanned = Arc::clone(&bytes);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let found = cx.background_spawn(async move { scan(&bytes, &how) }).await;
            in_window(&this, cx, |book, _, cx| {
                // Only this dialog's latest scan lands.
                let Some(Dialog::Import(pending)) = &mut book.dialog else {
                    return;
                };
                if pending.asked != asked || !Arc::ptr_eq(&pending.bytes, &scanned) {
                    return;
                }
                log::info!(
                    "table: import scan: {} columns, broken {}",
                    found.columns.len(),
                    found.broken
                );
                (pending.rows, pending.how.columns) = (found.rows, found.columns);
                (pending.broken, pending.reading) = (found.broken, false);
                cx.notify();
            })
        })
        .detach_and_log_err(cx);
    }

    /// Imports the file as the dialog says.
    pub(crate) fn import_asked(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Import(pending)) = self.dialog.take_if(|dialog| {
            matches!(dialog, Dialog::Import(pending) if !pending.reading && !pending.broken)
        }) else {
            return;
        };
        let how = &pending.how;
        log::info!(
            "table: imports as {:?}, {:?}, quote {:?}",
            how.encoding,
            how.separator,
            how.quote
        );
        let name = ely_office::files::file_name(&pending.path);
        self.import_text(&name, Arc::clone(&pending.bytes), pending.how.clone(), cx);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_read_as_text_and_numbers_stay_numbers() {
        assert!(is_code("00123") && is_code("+8613800000000"));
        assert!(is_code("12345678901234567890"));
        assert!(is_code("-12345678901234567890") && is_code("-007"));
        assert!(!is_code("123") && !is_code("0") && !is_code("1.50") && !is_code("+"));
        assert!(!is_code("-5") && !is_code("+-1"));

    }

    #[test]
    fn guesses_follow_the_bytes_and_the_name() {
        let csv = Path::new("a.csv");
        assert_eq!(guess_separator(csv, b"a;b;c,d\n1;2"), ';');
        assert_eq!(guess_separator(csv, b"one\n"), ',');
        assert_eq!(guess_separator(Path::new("a.TSV"), b"a,b"), '\t');
        assert_eq!(guess_encoding("名称".as_bytes(), Encoding::Gb18030), Encoding::Utf8);
        assert_eq!(guess_encoding(&[0xC3, 0xFB], Encoding::Gb18030), Encoding::Gb18030);
        assert_eq!(guess_encoding(&[0xC3, 0xFB], Encoding::Windows1252), Encoding::Windows1252);
        assert_eq!(guess_encoding(b"Cr\xe8me;br\xfbl\xe9e\n", Encoding::Windows1252), Encoding::Windows1252);
        assert_eq!(guess_encoding(b"Gr\xfc\xdfe;K\xf6ln\n", Encoding::Windows1252), Encoding::Windows1252);
        assert_eq!(guess_encoding(&[0xFF, 0xFE, b'a', 0], Encoding::Gb18030), Encoding::Utf16);
    }

    #[test]
    fn each_language_names_its_windows_code_page() {
        // Excel saves CSV in the system's ANSI code page.
        let pages = [
            ("de-DE", Encoding::Windows1252),
            ("en-US", Encoding::Windows1252),
            ("es-ES", Encoding::Windows1252),
            ("fr-FR", Encoding::Windows1252),
            ("ja-JP", Encoding::ShiftJis),
            ("ko-KR", Encoding::Windows949),
            ("vi-VN", Encoding::Windows1258),
            ("zh-CN", Encoding::Gb18030),
        ];
        let listed: Vec<_> = ely_platform::settings::LANGUAGES.iter().map(|(tag, _)| *tag).collect();
        assert_eq!(pages.map(|(tag, _)| tag).to_vec(), listed, "a new language picks its page");
        for (tag, page) in pages {
            assert_eq!(legacy_encoding(tag), page, "{tag}");
        }
    }

    #[test]
    fn shift_jis_reads_as_windows_932() {
        // Windows-932 alone maps ①, ㈱ and fullwidth tilde.
        let excel = [0x87, 0x40, 0x87, 0x8A, 0x81, 0x60, 0x93, 0xFA, 0x96, 0x7B];
        assert_eq!(decode(&excel, Encoding::ShiftJis).as_deref(), Some("①㈱～日本"));
        assert_eq!(guess_encoding(&excel, legacy_encoding("ja-JP")), Encoding::ShiftJis);
    }

    #[test]
    fn scans_read_every_row_and_refuse_broken_bytes() {
        let mut how = Import {
            separator: ',',
            quote: Some('"'),
            encoding: Encoding::Utf8,
            columns: Vec::new(),
        };
        let mut text = "Name,Note\n\"Doe, J\",1\n".to_string();
        text.push_str(&"Ada,123\n".repeat(20_000));
        text.push_str("Bo,00123\n");
        let found = scan(text.as_bytes(), &how);
        assert_eq!(found.rows.len(), SHOWN);
        assert_eq!(found.rows[1], ["Doe, J", "1"]);
        assert_eq!(found.columns, [Column::Standard, Column::Text], "the last row counts");
        let long = format!("{},x\n", "a".repeat(100_000));
        assert_eq!(scan(long.as_bytes(), &how).rows[0][0].chars().count(), CELL);
        assert!(scan(b"A\xffB\n", &how).broken, "no silent replacement");
        let calc = scan(b"Id,Note\n \"00123\",x\n\n\"a \"\"b\"\"\"c,\"two\nlines\"\r\n", &how);
        assert_eq!(calc.rows[1], ["00123", "x"], "blanks before a quote go");
        assert_eq!(calc.rows[2], [""], "an empty line is a row");
        assert_eq!(calc.rows[3], ["a \"b\"c", "two\nlines"]);
        assert_eq!(calc.columns, [Column::Text, Column::Standard]);
        how.quote = None;
        assert_eq!(scan(b"\"Doe, J\"\n", &how).rows[0], ["\"Doe", " J\""]);
        how.encoding = Encoding::Latin1;
        assert_eq!(scan(&[b'c', 0xE9], &how).rows[0], ["cé"]);
    }
}
