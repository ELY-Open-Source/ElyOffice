//! The kernel's CSV filter options (TABLE-12, 13).

/// How the kernel reads one column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Column {
    Standard,
    Text,
    DateMdy,
    DateDmy,
    DateYmd,
    Skip,
}

pub const COLUMNS: [Column; 6] = [
    Column::Standard,
    Column::Text,
    Column::DateMdy,
    Column::DateDmy,
    Column::DateYmd,
    Column::Skip,
];

impl Column {
    fn code(self) -> u8 {
        match self {
            Column::Standard => 1,
            Column::Text => 2,
            Column::DateMdy => 3,
            Column::DateDmy => 4,
            Column::DateYmd => 5,
            Column::Skip => 9,
        }
    }
}

/// Text encodings the import offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Utf16,
    Gb18030,
    Big5,
    ShiftJis,
    Windows949,
    Windows1252,
    Windows1251,
    Windows1258,
    Latin1,
}

pub const ENCODINGS: [Encoding; 10] = [
    Encoding::Utf8,
    Encoding::Utf16,
    Encoding::Gb18030,
    Encoding::Big5,
    Encoding::ShiftJis,
    Encoding::Windows949,
    Encoding::Windows1252,
    Encoding::Windows1251,
    Encoding::Windows1258,
    Encoding::Latin1,
];

impl Encoding {
    /// The label people know it by.
    pub fn label(self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Utf16 => "UTF-16",
            Encoding::Gb18030 => "GB18030",
            Encoding::Big5 => "Big5",
            Encoding::ShiftJis => "Shift_JIS",
            Encoding::Windows949 => "Windows-949",
            Encoding::Windows1252 => "Windows-1252",
            Encoding::Windows1251 => "Windows-1251",
            Encoding::Windows1258 => "Windows-1258",
            Encoding::Latin1 => "ISO-8859-1",
        }
    }

    /// The kernel's `rtl_TextEncoding`.
    fn kernel(self) -> u16 {
        match self {
            Encoding::Utf8 => 76,
            Encoding::Utf16 => 0xFFFF,
            Encoding::Gb18030 => 85,
            Encoding::Big5 => 68,
            Encoding::ShiftJis => 64,
            Encoding::Windows949 => 62,
            Encoding::Windows1252 => 1,
            Encoding::Windows1251 => 34,
            Encoding::Windows1258 => 40,
            Encoding::Latin1 => 12,
        }
    }
}

/// US English numbers: `.` decimal, `,` thousands.
const LANGUAGE: u16 = 1033;

/// How a text file becomes a sheet.
#[derive(Clone, Debug, PartialEq)]
pub struct Import {
    pub separator: char,
    pub quote: Option<char>,
    pub encoding: Encoding,
    /// Kinds of the first columns; the rest are standard.
    pub columns: Vec<Column>,
}

impl Import {
    /// The kernel's filter options for this import.
    pub fn filter(&self) -> String {
        let columns: Vec<String> = self
            .columns
            .iter()
            .enumerate()
            .map(|(index, column)| format!("{}/{}", index + 1, column.code()))
            .collect();
        // Token order: sc/source/ui/dbgui/asciiopt.cxx.
        [
            u32::from(self.separator).to_string(),
            self.quote.map_or(0, u32::from).to_string(),
            self.encoding.kernel().to_string(),
            "1".into(),
            columns.join("/"),
            LANGUAGE.to_string(),
            // Quoted as text, special numbers, two export flags, trim.
            "false,false,false,false,false".into(),
            "0".into(),
            // Formulas stay text; no BOM; scientific numbers read.
            "false,false,true".into(),
        ]
        .join(",")
    }
}

/// Options writing the view's sheet as shown, UTF-8.
pub fn export(separator: char) -> String {
    [
        u32::from(separator).to_string(),
        "34".into(),
        Encoding::Utf8.kernel().to_string(),
        "1".into(),
        "1/1".into(),
        LANGUAGE.to_string(),
        // Quote all text, special numbers, as shown, formulas, trim.
        "false,false,true,false,false".into(),
        "0".into(),
        "false,false,false".into(),
    ]
    .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_name_every_token() {
        let import = Import {
            separator: '\t',
            quote: Some('"'),
            encoding: Encoding::Gb18030,
            columns: vec![Column::Text, Column::Standard, Column::DateYmd],
        };
        assert_eq!(
            import.filter(),
            "9,34,85,1,1/2/2/1/3/5,1033,false,false,false,false,false,0,false,false,true"
        );
        let bare = Import {
            quote: None,
            columns: Vec::new(),
            ..import
        };
        assert!(bare.filter().starts_with("9,0,85,1,,1033,"));
    }

    #[test]
    fn exports_write_values_as_shown() {
        assert_eq!(
            export(';'),
            "59,34,76,1,1/1,1033,false,false,true,false,false,0,false,false,false"
        );
    }
}
