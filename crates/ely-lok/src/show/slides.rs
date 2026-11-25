//! Every slide's name, layout, title, notes and boxes, by index.

use std::io;

use serde_json::Value;

/// All slides: name, layout id, title, hidden, notes, boxes.
pub const SLIDES: &str = ".uno:ElySlides";

/// One slide as the deck lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slide {
    pub name: String,
    /// Impress's AutoLayout id, as in [`super::LAYOUTS`].
    pub layout: u32,
    pub title: String,
    pub hidden: bool,
    pub notes: String,
    /// Placeholders there, by name; true when the text mixes looks.
    pub boxes: Vec<(String, bool)>,
}

/// Parses `.uno:ElySlides`.
pub fn slides(json: &str) -> io::Result<Vec<Slide>> {
    let value: Value = serde_json::from_str(json).map_err(io::Error::other)?;
    let bad = |what: &str, item: &Value| io::Error::other(format!("bad {what} in {item}"));
    let text = |item: &Value, key: &str| item[key].as_str().map(str::to_string).ok_or_else(|| bad(key, item));
    let list = value["slides"].as_array().ok_or_else(|| bad("slides", &value))?;
    list.iter()
        .map(|item| {
            let layout = item["layout"].as_u64().and_then(|id| u32::try_from(id).ok());
            Ok(Slide {
                name: text(item, "name")?,
                layout: layout.ok_or_else(|| bad("layout", item))?,
                title: text(item, "title")?,
                hidden: item["hidden"].as_bool().ok_or_else(|| bad("hidden", item))?,
                notes: text(item, "notes")?,
                boxes: (item["boxes"].as_object().ok_or_else(|| bad("boxes", item))?.iter())
                    .map(|(name, rich)| Ok((name.clone(), rich.as_bool().ok_or_else(|| bad("boxes", item))?)))
                    .collect::<io::Result<_>>()?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Slide, slides};

    #[test]
    fn each_slide_keeps_its_fields() {
        let json = r#"{"commandName":".uno:ElySlides","slides":[
            {"name":"Slide 1","layout":0,"title":"Hello\nworld","hidden":false,"notes":"Say hi",
             "boxes":{"title":false,"body":true,"notes":false}},
            {"name":"Intro","layout":20,"title":"","hidden":true,"notes":"","boxes":{}}]}"#;
        let read = slides(json).expect("parsed");
        let boxes = vec![("title".into(), false), ("body".into(), true), ("notes".into(), false)];
        let title = "Hello\nworld".into();
        let first = Slide { name: "Slide 1".into(), layout: 0, title, hidden: false, notes: "Say hi".into(), boxes };
        assert_eq!(read[0], first);
        assert_eq!((read[1].layout, read[1].title.as_str(), read[1].hidden, read[1].boxes.len()), (20, "", true, 0));
    }

    #[test]
    fn a_missing_field_is_an_error() {
        assert!(slides(r#"{"slides":[{"name":"a","layout":1,"hidden":false}]}"#).is_err(), "no title");
        let no_boxes = r#"{"slides":[{"name":"a","layout":1,"title":"","hidden":false,"notes":""}]}"#;
        assert!(slides(no_boxes).is_err(), "no boxes");
        assert!(slides(r#"{"slides":[{"name":"a","layout":-1,"title":"","hidden":false}]}"#).is_err());
        assert!(slides("{}").is_err(), "no list");
    }
}
