//! A presentation's design read from the kernel (SHOW-01, 02).

use std::io;

use serde_json::Value;

mod check;
mod layers;
mod media;
mod motion;
mod objects;
mod parts;
mod playlist;
mod slides;
pub use check::*;
pub use layers::*;
pub use media::*;
pub use motion::*;
pub use objects::*;
pub use parts::*;
pub use playlist::*;
pub use slides::*;

/// Masters, layouts and color sets of the shown slide.
pub const DESIGN: &str = ".uno:SlideDesign";

/// Layouts the slide panel offers, as kernel AutoLayout numbers.
pub const LAYOUTS: [u32; 12] = [20, 0, 1, 3, 19, 32, 12, 15, 14, 16, 18, 34];

/// The shown slide's design and the choices.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Design {
    pub masters: Vec<String>,
    pub master: String,
    /// The view edits masters, not slides.
    pub master_view: bool,
    pub layout: u32,
    /// The master's color set; empty without one.
    pub theme: String,
    pub themes: Vec<String>,
}

fn texts(value: &Value, key: &str) -> io::Result<Vec<String>> {
    value[key]
        .as_array()
        .ok_or_else(|| io::Error::other(format!("no {key} in {value}")))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| io::Error::other(format!("bad {key} in {value}")))
        })
        .collect()
}

fn text(value: &Value, key: &str) -> io::Result<String> {
    value[key]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| io::Error::other(format!("no {key} in {value}")))
}

/// Parses `.uno:SlideDesign`.
pub fn design(json: &str) -> io::Result<Design> {
    let value: Value = serde_json::from_str(json)
        .map_err(|error| io::Error::other(format!("bad design: {error}")))?;
    let layout = value["layout"]
        .as_u64()
        .and_then(|layout| u32::try_from(layout).ok())
        .ok_or_else(|| io::Error::other(format!("no layout in {value}")))?;
    Ok(Design {
        masters: texts(&value, "masters")?,
        master: text(&value, "master")?,
        master_view: value["masterView"]
            .as_bool()
            .ok_or_else(|| io::Error::other(format!("no masterView in {value}")))?,
        layout,
        theme: text(&value, "theme")?,
        themes: texts(&value, "themes")?,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn designs_parse_and_fail_on_gaps() {
        let json = r#"{"masters":["A","B"],"master":"B","masterView":false,"layout":1,"theme":"","themes":["LibreOffice"]}"#;
        let design = super::design(json).unwrap();
        assert_eq!((design.master.as_str(), design.layout), ("B", 1));
        assert!(super::design(&json.replace(r#""layout":1,"#, "")).is_err());
    }
}
