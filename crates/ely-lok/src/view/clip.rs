//! The clipboard for LokView (LOK-06, TABLE-08).

use std::{
    io,
    sync::atomic::{AtomicU64, Ordering},
};

use ely_platform::clipboard::{self, Clip};
use gpui::{Action, Keystroke};

use super::input::{Copy, Cut, Paste};
use crate::{Doc, Worker};

/// A clipboard chord's action; the kernel would copy privately.
pub(super) fn chord(keystroke: &Keystroke) -> Option<Box<dyn Action>> {
    let keys = &keystroke.modifiers;
    if keys.alt || keys.function {
        return None;
    }
    let secondary = if cfg!(target_os = "macos") {
        keys.platform && !keys.control
    } else {
        keys.control && !keys.platform
    };
    let bare = !keys.control && !keys.platform;
    let action: Box<dyn Action> = match (keystroke.key.as_str(), keys.shift) {
        ("c", false) if secondary => Box::new(Copy),
        ("x", false) if secondary => Box::new(Cut),
        ("v", false) if secondary => Box::new(Paste),
        ("insert", false) if keys.control && !keys.platform => Box::new(Copy),
        ("delete", true) if bare => Box::new(Cut),
        ("insert", true) if bare => Box::new(Paste),
        _ => return None,
    };
    Some(action)
}

/// The clipboard payload for copied LOK flavors.
pub(super) fn clip_of(flavors: Vec<(String, Vec<u8>)>) -> Option<Clip> {
    let take = |mime: &str| {
        flavors
            .iter()
            .find(|(name, _)| name == mime)
            .map(|(_, data)| data.clone())
    };
    let text =
        take("text/plain;charset=utf-8").map(|data| String::from_utf8_lossy(&data).into_owned());
    let png = take("image/png");
    if let (Some(png), None) = (&png, &text) {
        return Some(Clip::Png(png.clone()));
    }
    let html = take("text/html").map(|html| String::from_utf8_lossy(&html).into_owned());
    match (html, text, png) {
        // Cells come with their picture, for chat boxes (#141).
        (Some(html), Some(text), Some(png)) => Some(Clip::Pictured { html, text, png }),
        (Some(html), text, _) => Some(Clip::Html {
            html,
            text: text.unwrap_or_default(),
        }),
        (None, Some(text), _) => Some(Clip::Text(text)),
        (None, None, _) => None,
    }
}

/// The LOK flavor for a clipboard payload.
pub(super) fn flavor_of(clip: Clip) -> (&'static str, Vec<u8>) {
    match clip {
        Clip::Text(text) => ("text/plain;charset=utf-8", text.into_bytes()),
        Clip::Html { html, .. } | Clip::Pictured { html, .. } => ("text/html", html.into_bytes()),
        Clip::Png(png) => ("image/png", png),
    }
}

/// Copies out, marked; a cut deletes after the write.
pub(super) fn copy_out(
    worker: &mut Worker,
    doc: &Doc,
    cut: bool,
    write: impl FnOnce(&Clip) -> io::Result<()>,
) -> io::Result<Option<String>> {
    let Some(mut clip) = clip_of(worker.copy(doc)?) else {
        return Ok(None);
    };
    let mark = if let Clip::Html { html, .. } | Clip::Pictured { html, .. } = &mut clip {
        static COPIES: AtomicU64 = AtomicU64::new(0);
        let mark = format!(
            "{}-{}",
            std::process::id(),
            COPIES.fetch_add(1, Ordering::SeqCst)
        );
        html.push_str(&format!("<!--{MARK}{mark}-->"));
        Some(mark)
    } else {
        None
    };
    write(&clip)?;
    if cut {
        worker.cut(doc)?;
    }
    Ok(mark)
}

/// Ely's copies end their HTML with this and a mark.
const MARK: &str = "ely-copy:";

/// The mark of Ely's copy in `html`, if any.
pub(super) fn mark_of(html: &str) -> Option<&str> {
    let start = html.rfind(&format!("<!--{MARK}"))? + 4 + MARK.len();
    let end = html[start..].find("-->")?;
    Some(&html[start..start + end])
}

/// Pastes the system clipboard.
pub(super) fn paste(worker: &mut Worker, doc: &Doc) -> io::Result<()> {
    let Some(clip) = clipboard::read().map_err(|error| io::Error::other(format!("{error:#}")))? else {
        return Ok(());
    };
    let (mime, data) = flavor_of(clip);
    worker.paste(doc, mime, &data)
}
