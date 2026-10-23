//! The fields every app reads from the shared file.

use std::io;

use serde_json::{Map, Value};

use super::Store;

/// Light, dark, or as the system is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

/// The interface language; System follows the OS.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    System,
    English,
    Chinese,
}

/// Interface motion; System follows the OS setting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Motion {
    #[default]
    System,
    Reduced,
    Full,
}

pub const APPEARANCE: &str = "appearance";
pub const LANGUAGE: &str = "language";
pub const MOTION: &str = "motion";
pub const OMARCHY_SYNC: &str = "omarchy_sync";
pub const AUTHOR: &str = "author";
pub const CHECK_UPDATES: &str = "check_updates";
pub const UI_FONT: &str = "ui_font";
pub const UI_TEXT_SIZE: &str = "ui_text_size";
/// Interface text in pixels: least, most, default.
pub const UI_TEXT_SIZES: (u32, u32, u32) = (12, 16, 13);
pub const DOC_ZOOM: &str = "doc_zoom";
/// The percents a document may open at.
pub const DOC_ZOOMS: [u32; 6] = [50, 75, 100, 125, 150, 200];

/// How Doc, Table, Show and PDF open a document.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DocZoom {
    /// Each app's own: 100%, or a whole slide or width.
    #[default]
    Auto,
    Percent(u32),
}

pub const TWO_SIDED: &str = "two_sided";

/// How the print dialog starts out turning pages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sides {
    /// Whatever the printer is set to.
    #[default]
    Printer,
    One,
    /// Bound on the long edge.
    Long,
    /// Bound on the short edge.
    Short,
}

impl Sides {
    pub const ALL: [(Self, &'static str); 4] = [(Self::Printer, "printer"), (Self::One, "one"), (Self::Long, "long"), (Self::Short, "short")];
}

impl DocZoom {
    /// As the file stores it.
    pub fn value(self) -> Value {
        match self {
            DocZoom::Auto => Value::String("auto".into()),
            DocZoom::Percent(percent) => percent.into(),
        }
    }
}

/// The shared settings; absent fields take their defaults.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shared {
    pub appearance: Appearance,
    pub language: Language,
    pub motion: Motion,
    pub omarchy_sync: bool,
    /// Signs comments and changes; empty means the account name.
    pub author: String,
    /// Asks for a newer release at launch; off unless chosen.
    pub check_updates: bool,
    /// A system font family; empty is the theme's own.
    pub ui_font: String,
    /// Pixels of the interface's body text.
    pub ui_text_size: u32,
    pub doc_zoom: DocZoom,
    pub two_sided: Sides,
}

impl Default for Shared {
    fn default() -> Self {
        Self {
            appearance: Appearance::System,
            language: Language::System,
            motion: Motion::System,
            omarchy_sync: true,
            author: String::new(),
            check_updates: false,
            ui_font: String::new(),
            ui_text_size: UI_TEXT_SIZES.2,
            doc_zoom: DocZoom::Auto,
            two_sided: Sides::Printer,
        }
    }
}

impl Appearance {
    pub const ALL: [(Self, &'static str); 3] = [(Self::System, "system"), (Self::Light, "light"), (Self::Dark, "dark")];
}

impl Language {
    pub const ALL: [(Self, &'static str); 3] = [(Self::System, "system"), (Self::English, "en-US"), (Self::Chinese, "zh-CN")];
}

impl Motion {
    pub const ALL: [(Self, &'static str); 3] = [(Self::System, "system"), (Self::Reduced, "reduce"), (Self::Full, "full")];
}

/// The stored word for `choice` among `all`.
pub fn word<T: PartialEq>(all: &[(T, &'static str)], choice: &T) -> &'static str {
    all.iter().find(|(each, _)| each == choice).map(|(_, word)| *word).expect("every choice has a word")
}

impl Shared {
    /// The shared file as read now.
    pub fn load() -> io::Result<Self> {
        Self::from_fields(&Store::shared()?.read()?)
    }

    /// Known fields, typed; a wrong value is an error.
    pub fn from_fields(fields: &Map<String, Value>) -> io::Result<Self> {
        let defaults = Self::default();
        Ok(Self {
            appearance: pick(fields, APPEARANCE, &Appearance::ALL)?.unwrap_or(defaults.appearance),
            language: pick(fields, LANGUAGE, &Language::ALL)?.unwrap_or(defaults.language),
            motion: pick(fields, MOTION, &Motion::ALL)?.unwrap_or(defaults.motion),
            check_updates: match fields.get(CHECK_UPDATES) {
                None => defaults.check_updates,
                Some(Value::Bool(on)) => *on,
                Some(other) => return Err(wrong(CHECK_UPDATES, other)),
            },
            omarchy_sync: match fields.get(OMARCHY_SYNC) {
                None => defaults.omarchy_sync,
                Some(Value::Bool(on)) => *on,
                Some(other) => return Err(wrong(OMARCHY_SYNC, other)),
            },
            author: match fields.get(AUTHOR) {
                None => defaults.author,
                Some(Value::String(name)) => name.trim().to_string(),
                Some(other) => return Err(wrong(AUTHOR, other)),
            },
            ui_font: match fields.get(UI_FONT) {
                None => defaults.ui_font,
                Some(Value::String(family)) => family.trim().to_string(),
                Some(other) => return Err(wrong(UI_FONT, other)),
            },
            ui_text_size: match fields.get(UI_TEXT_SIZE) {
                None => defaults.ui_text_size,
                Some(value) => value
                    .as_u64()
                    .filter(|size| (UI_TEXT_SIZES.0 as u64..=UI_TEXT_SIZES.1 as u64).contains(size))
                    .ok_or_else(|| wrong(UI_TEXT_SIZE, value))? as u32,
            },
            doc_zoom: match fields.get(DOC_ZOOM) {
                None => defaults.doc_zoom,
                Some(Value::String(word)) if word == "auto" => DocZoom::Auto,
                Some(value) => value
                    .as_u64()
                    .and_then(|percent| DOC_ZOOMS.into_iter().find(|listed| *listed as u64 == percent))
                    .map(DocZoom::Percent)
                    .ok_or_else(|| wrong(DOC_ZOOM, value))?,
            },
            two_sided: pick(fields, TWO_SIDED, &Sides::ALL)?.unwrap_or(defaults.two_sided),
        })
    }
}

fn pick<T: Copy>(fields: &Map<String, Value>, key: &str, all: &[(T, &'static str)]) -> io::Result<Option<T>> {
    match fields.get(key) {
        None => Ok(None),
        Some(Value::String(word)) => match all.iter().find(|(_, each)| each == word) {
            Some((choice, _)) => Ok(Some(*choice)),
            None => Err(wrong(key, &Value::String(word.clone()))),
        },
        Some(other) => Err(wrong(key, other)),
    }
}

fn wrong(key: &str, value: &Value) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("settings: {key} cannot be {value}"))
}
