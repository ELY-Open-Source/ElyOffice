//! The system clipboard: text, HTML and PNG images (LOK-06).

use std::sync::Mutex;

use anyhow::{Context, Result};
use arboard::{Clipboard, Error, ImageData};

/// One clipboard payload.
#[derive(Clone, Debug, PartialEq)]
pub enum Clip {
    Text(String),
    /// Rich text or tables, with a plain fallback.
    Html {
        html: String,
        text: String,
    },
    Png(Vec<u8>),
}

/// One handle for the process; Linux serves from it.
static BOARD: Mutex<Option<Clipboard>> = Mutex::new(None);

fn with_board<T>(call: impl FnOnce(&mut Clipboard) -> Result<T>) -> Result<T> {
    let mut board = BOARD.lock().expect("the clipboard lock");
    if board.is_none() {
        *board = Some(Clipboard::new().context("the system clipboard opens")?);
    }
    call(board.as_mut().expect("opened above"))
}

/// Puts `clip` on the system clipboard.
pub fn write(clip: &Clip) -> Result<()> {
    with_board(|board| {
        match clip {
            Clip::Text(text) => board.set_text(text.as_str())?,
            Clip::Html { html, text } => board.set_html(html.as_str(), Some(text.as_str()))?,
            Clip::Png(png) => {
                let image = image::load_from_memory(png)
                    .context("the copied png decodes")?
                    .to_rgba8();
                board.set_image(ImageData {
                    width: image.width() as usize,
                    height: image.height() as usize,
                    bytes: image.into_raw().into(),
                })?
            }
        }
        log::info!("clipboard: wrote {}", kind(clip));
        Ok(())
    })
}

/// The richest payload on the clipboard: HTML, image, then text.
pub fn read() -> Result<Option<Clip>> {
    let clip = with_board(richest)?;
    log::info!("clipboard: read {}", clip.as_ref().map_or("nothing", kind));
    Ok(clip)
}

fn richest(board: &mut Clipboard) -> Result<Option<Clip>> {
    let text = absent_ok(board.get_text())?;
    if let Some(html) = html_of(board)? {
        let text = text.unwrap_or_default();
        return Ok(Some(Clip::Html { html, text }));
    }
    if let Some(image) = absent_ok(board.get_image())? {
        let rgba = image::RgbaImage::from_raw(
            image.width as u32,
            image.height as u32,
            image.bytes.into_owned(),
        )
        .context("the clipboard image has its size")?;
        let mut png = Vec::new();
        rgba.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .context("the clipboard image encodes")?;
        return Ok(Some(Clip::Png(png)));
    }
    Ok(text.map(Clip::Text))
}

/// The clipboard's HTML, if any; gpui's reads omit it.
pub fn html() -> Result<Option<String>> {
    with_board(html_of)
}

fn html_of(board: &mut Clipboard) -> Result<Option<String>> {
    // arboard reads absent HTML on Windows as an error.
    #[cfg(windows)]
    {
        let format = clipboard_win::register_format("HTML Format")
            .context("the HTML clipboard format registers")?;
        if !clipboard_win::is_format_avail(format.get()) {
            return Ok(None);
        }
    }
    absent_ok(board.get().html())
}

/// Empty or other-format clipboards read as None.
fn absent_ok<T>(outcome: Result<T, Error>) -> Result<Option<T>> {
    match outcome {
        Ok(value) => Ok(Some(value)),
        Err(Error::ContentNotAvailable) => Ok(None),
        Err(error) => Err(error).context("the clipboard reads"),
    }
}

fn kind(clip: &Clip) -> &'static str {
    match clip {
        Clip::Text(_) => "text",
        Clip::Html { .. } => "html",
        Clip::Png(_) => "png",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes then reads the real system clipboard.
    #[test]
    fn each_kind_round_trips() {
        if std::env::var_os("ELY_CLIPBOARD_TEST").is_none() {
            eprintln!("SKIP: set ELY_CLIPBOARD_TEST=1; this replaces the clipboard");
            return;
        }
        let text = Clip::Text("plain 中文".into());
        write(&text).expect("text writes");
        assert_eq!(read().expect("text reads"), Some(text));
        assert_eq!(html().expect("no html reads"), None);
        let html = Clip::Html {
            html: "<table><tr><td>a</td></tr></table>".into(),
            text: "a".into(),
        };
        write(&html).expect("html writes");
        let Some(Clip::Html { html: back, text }) = read().expect("html reads") else {
            panic!("html comes back as html");
        };
        assert!(back.contains("<td>a</td>"), "the table survives: {back}");
        assert_eq!(text, "a");
        let mut png = Vec::new();
        image::RgbaImage::from_pixel(3, 2, image::Rgba([0, 0, 255, 255]))
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("the png encodes");
        write(&Clip::Png(png)).expect("png writes");
        let Some(Clip::Png(back)) = read().expect("png reads") else {
            panic!("an image comes back as png");
        };
        let back = image::load_from_memory(&back).expect("decodes").to_rgba8();
        assert_eq!((back.width(), back.height()), (3, 2));
        assert_eq!(back.get_pixel(0, 0).0, [0, 0, 255, 255]);
    }
}
