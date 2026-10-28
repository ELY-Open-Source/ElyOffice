//! A slide drawn as slideshow layers (SHOW-09).

use std::{collections::HashMap, io};

use serde_json::Value;

use crate::ipc::Unpack;

/// What a layer is.
#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    /// Still content: background, master, slide, field text.
    Still,
    /// One animated object; `hash` matches its effects.
    Moving {
        hash: String,
        visible: bool,
        /// x, y, width, height on the slide, 1/100 mm.
        bounds: (i64, i64, i64, i64),
    },
}

/// One layer with tile-format pixels.
pub struct Layer {
    pub part: Part,
    pub pixels: Vec<u8>,
}

/// A slide's layers, bottom first, at one pixel size.
pub struct Layers {
    pub size: (u32, u32),
    pub list: Vec<Layer>,
}

/// What the kernel sent for one layer.
enum Sent {
    Layer(Part),
    /// Where a master field goes, by object hash.
    Placeholder(String),
    /// A field's text for this slide, by object hash.
    FieldText(String),
}

/// Pixels that fill `width` by `height`, four bytes each.
pub(crate) fn sized(pixels: &[u8], (width, height): (u32, u32)) -> io::Result<()> {
    let asked = u64::from(width) * u64::from(height) * 4;
    if pixels.len() as u64 != asked {
        return Err(io::Error::other(format!("{} bytes for {width}x{height}", pixels.len())));
    }
    Ok(())
}

fn sent(json: &Value) -> io::Result<Sent> {
    let bad = || io::Error::other(format!("bad layer {json}"));
    let content = &json["content"];
    let hash = || (content["hash"].as_str().map(str::to_string)).ok_or_else(bad);
    if json["group"].as_str() == Some("TextFields") {
        return Ok(Sent::FieldText(hash()?));
    }
    match json["type"].as_str() {
        Some("bitmap") => Ok(Sent::Layer(Part::Still)),
        Some("animated") => {
            let at = |key: &str| content["bounds"][key].as_i64().ok_or_else(bad);
            Ok(Sent::Layer(Part::Moving {
                hash: hash()?,
                visible: content["initVisible"].as_bool().ok_or_else(bad)?,
                bounds: (at("x")?, at("y")?, at("width")?, at("height")?),
            }))
        }
        Some("placeholder") => Ok(Sent::Placeholder(hash()?)),
        _ => Err(bad()),
    }
}

/// Parses the worker's slide layers reply; fields go in place.
pub fn layers(payload: &[u8]) -> io::Result<Layers> {
    let mut unpack = Unpack::new(payload);
    let size = (u32::try_from(unpack.u64()?), u32::try_from(unpack.u64()?));
    let (Ok(width), Ok(height)) = size else {
        return Err(io::Error::other("bad layer size"));
    };
    let count = unpack.u64()?;
    let mut sent_list = Vec::new();
    for _ in 0..count {
        let json: Value = serde_json::from_str(&unpack.str()?).map_err(io::Error::other)?;
        let pixels = (unpack.u64()? != 0).then(|| unpack.blob()).transpose()?;
        let kind = sent(&json)?;
        if pixels.is_some() == matches!(kind, Sent::Placeholder(_)) {
            return Err(io::Error::other(format!("pixels do not fit layer {json}")));
        }
        if let Some(pixels) = &pixels {
            sized(pixels, (width, height))?;
        }
        sent_list.push((kind, pixels));
    }
    let mut texts: HashMap<String, Vec<u8>> = HashMap::new();
    for (kind, pixels) in &mut sent_list {
        if let Sent::FieldText(hash) = kind {
            texts.insert(hash.clone(), pixels.take().expect("checked above"));
        }
    }
    // A field hidden on this slide has no text.
    let list = (sent_list.into_iter())
        .filter_map(|(kind, pixels)| match kind {
            Sent::Layer(part) => Some(Layer {
                part,
                pixels: pixels.expect("checked above"),
            }),
            Sent::Placeholder(hash) => texts.remove(&hash).map(|pixels| Layer {
                part: Part::Still,
                pixels,
            }),
            Sent::FieldText(_) => None,
        })
        .collect();
    Ok(Layers {
        size: (width, height),
        list,
    })
}

#[cfg(test)]
mod tests {
    use crate::ipc::Pack;

    fn reply(pixels: &[u8]) -> Vec<u8> {
        let layer = r#"{"type":"bitmap"}"#;
        Pack::new().u64(2).u64(1).u64(1).str(layer).u64(1).blob(pixels).bytes()
    }

    #[test]
    fn a_layer_of_another_size_is_refused() {
        assert_eq!(super::layers(&reply(&[0; 8])).expect("fits").list.len(), 1);
        let said = super::layers(&reply(&[0; 4])).err().expect("refused");
        assert!(said.to_string().contains("4 bytes for 2x1"), "{said}");
    }
}
