use std::io;

use super::with_doc;
use crate::commands::{Arg, Check, Command};

use crate::ffi::Document;
use crate::history::Step;
use crate::ipc::{self, Pack, Unpack};

/// `.uno:Paste` arguments, as `doc_paste` sets them.
const PASTE_ARGS: &str = r#"{"AnchorType":{"type":"unsigned short","value":"1"},"IgnoreComments":{"type":"boolean","value":"true"},"SkipDetection":{"type":"boolean","value":"true"}}"#;
/// The clipboard flavors Ely moves (LOK-06).
const CLIP_MIMES: [&str; 3] = ["text/html", "text/plain;charset=utf-8", "image/png"];

/// A failed request as its error text.
fn invalid(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, what.to_string())
}

/// One tile and the generation it was painted at (LOK-03).
fn paint(unpack: &mut Unpack) -> io::Result<(u64, Vec<u8>)> {
    let id = unpack.u64()?;
    let (x, y, w, h) = (unpack.i64()?, unpack.i64()?, unpack.i64()?, unpack.i64()?);
    let (pw, ph) = (unpack.i64()?, unpack.i64()?);
    let geometry_ok = (1..=i32::MAX as i64).contains(&w)
        && (1..=i32::MAX as i64).contains(&h)
        && pw > 0
        && ph > 0;
    let len = (geometry_ok)
        .then(|| (pw as u64).checked_mul(ph as u64)?.checked_mul(4))
        .flatten()
        .filter(|len| *len <= (ipc::MAX_FRAME - 16) as u64);
    let Some(len) = len else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "bad tile geometry",
        ));
    };
    // Read first: an invalidation mid-paint must outdate it.
    let generation = crate::events::generation(id);
    let rect = (x as i32, y as i32, w as i32, h as i32);
    let (at, tile) = with_doc(id, |doc| {
        let mut tile = vec![0u8; len as usize];
        doc.paint_rect(&mut tile, (pw as i32, ph as i32), rect);
        ((doc.part(), doc.edit_mode()), tile)
    })?;
    crate::events::painted(id, at, rect);
    Ok((generation, tile))
}

/// One slide's whole picture at the asked pixels.
pub(super) fn part_painted(unpack: &mut Unpack) -> io::Result<Vec<u8>> {
    let (id, part, master) = (unpack.u64()?, unpack.u64()?, unpack.u64()? != 0);
    let (pw, ph) = (unpack.u64()?, unpack.u64()?);
    let len = (pw > 0 && ph > 0)
        .then(|| pw.checked_mul(ph)?.checked_mul(4))
        .flatten()
        .filter(|len| *len <= (ipc::MAX_FRAME - 16) as u64)
        .ok_or_else(|| invalid("bad part geometry"))?;
    let part = i32::try_from(part).map_err(|_| invalid("bad part"))?;
    let at = (part, i32::from(master));
    let (size, pixels) = with_doc(id, |doc| {
        if part >= doc.parts() {
            return Err(invalid("no such part"));
        }
        let mut pixels = vec![0u8; len as usize];
        doc.paint_part(&mut pixels, at, (pw as i32, ph as i32));
        Ok((doc.size(), pixels))
    })??;
    // The kernel counts the whole slide as painted.
    crate::events::painted(id, at, (0, 0, size.0 as i32, size.1 as i32));
    Ok(pixels)
}

/// A painted tile behind its generation.
pub(super) fn painted(unpack: &mut Unpack) -> io::Result<Vec<u8>> {
    let (generation, tile) = paint(unpack)?;
    let mut body = Pack::new().u64(generation).bytes();
    body.extend_from_slice(&tile);
    Ok(body)
}

/// The kernel's accelerators that close its frame or quit.
const CLOSING: [i64; 3] = [534, 771, 528];

/// A key press and release (LOK-06).
pub(super) fn key(unpack: &mut Unpack) -> io::Result<Vec<u8>> {
    let (id, char_code, key_code) = (unpack.u64()?, unpack.i64()?, unpack.i64()?);
    // Mod1 with W, F4 or Q closes our document.
    if key_code & 0x2000 != 0 && CLOSING.contains(&(key_code & 0x0fff)) {
        return Err(io::Error::other(format!("key {key_code} closes the kernel's frame")));
    }
    with_doc(id, |doc| {
        doc.post_key(0, char_code as i32, key_code as i32);
        doc.post_key(1, char_code as i32, key_code as i32);
    })?;
    Ok(Vec::new())
}

/// Turns `step` parts in order, or posts the arrow (-1).
pub(super) fn turn(unpack: &mut Unpack) -> io::Result<Vec<u8>> {
    let (id, step, char_code, key_code) = (unpack.u64()?, unpack.i64()?, unpack.i64()?, unpack.i64()?);
    // Earlier input's callbacks already landed here.
    if key_code != 0 && crate::events::picked(id) {
        with_doc(id, |doc| {
            doc.post_key(0, char_code as i32, key_code as i32);
            doc.post_key(1, char_code as i32, key_code as i32);
        })?;
        return Ok(Pack::new().i64(-1).bytes());
    }
    let shown = with_doc(id, |doc| {
        let (part, last) = (i64::from(doc.part()), i64::from(doc.parts()) - 1);
        let next = (part + step).clamp(0, last.max(0));
        if next != part {
            doc.set_part(next as i32);
        }
        next
    })?;
    Ok(Pack::new().i64(shown).bytes())
}

/// One mouse event: kind, x, y, count, buttons, modifiers.
pub(super) fn mouse(unpack: &mut Unpack) -> io::Result<Vec<u8>> {
    let id = unpack.u64()?;
    let mut event = [0i32; 6];
    for slot in &mut event {
        *slot = unpack.i64()? as i32;
    }
    if !(0..=2).contains(&event[0]) {
        return Err(invalid("bad mouse event"));
    }
    with_doc(id, |doc| doc.post_mouse(event))?;
    Ok(Vec::new())
}

/// A selected object's handle drag: 0 start, 1 end.
pub(super) fn graphic(unpack: &mut Unpack) -> io::Result<Vec<u8>> {
    let (id, kind, x, y) = (unpack.u64()?, unpack.i64()?, unpack.i64()?, unpack.i64()?);
    if !(0..=1).contains(&kind) {
        return Err(invalid("bad graphic selection"));
    }
    with_doc(id, |doc| {
        doc.set_graphic_selection(kind as i32, x as i32, y as i32)
    })?;
    Ok(Vec::new())
}

/// IME text: 0 composing, 2 commit (LOK-07).
pub(super) fn text(unpack: &mut Unpack) -> io::Result<Vec<u8>> {
    let (id, kind, text) = (unpack.u64()?, unpack.i64()?, unpack.str()?);
    if kind != 0 && kind != 2 {
        return Err(invalid("bad text event"));
    }
    with_doc(id, |doc| doc.post_text(kind as i32, &text))??;
    Ok(Vec::new())
}

/// How long a state check waits for the kernel.
const STATE_WAIT: std::time::Duration = std::time::Duration::from_secs(2);

/// Runs a table command and confirms it took (LOK-02, LOK-08).
pub(super) fn uno(unpack: &mut Unpack) -> io::Result<Vec<u8>> {
    let (id, name) = (unpack.u64()?, unpack.str()?);
    let values = (0..unpack.u64()?)
        .map(|_| unpack.str())
        .collect::<io::Result<Vec<_>>>()?;
    let values: Vec<&str> = values.iter().map(String::as_str).collect();
    let command = crate::commands::find(&name)?;
    let args = command.args_json(&values)?;
    with_doc(id, |doc| {
        run_checked(id, doc, command, &values, args.as_deref())
    })??;
    log::info!("lok-worker: ran {name}");
    Ok(Vec::new())
}

/// Dispatches `command` and applies its check.
fn run_checked(
    id: u64,
    doc: &Document,
    command: &Command,
    values: &[&str],
    args: Option<&str>,
) -> io::Result<()> {
    let refuse = |why: String| Err(io::Error::other(format!("{}: {why}", command.id)));
    for (arg, value) in command.args.iter().zip(values) {
        if matches!(arg, Arg::Font(_)) && !font_listed(doc, value)? {
            return refuse(format!("the kernel lists no font {value}"));
        }
    }
    if command.check == Check::History && doc.kind() == 1 {
        return sheet_history(id, doc, command, args);
    }
    let history = format!("{}Count", command.uno);
    let before = if command.check == Check::History {
        let count = history_count(doc, &history)?;
        if count == 0 {
            return Err(io::Error::other(format!("{}{}: {history} is 0", ipc::NOTHING, command.id)));
        }
        count
    } else {
        0
    };
    // Writer's font state shows the caret's script only.
    let fonts_before = match command.args.first() {
        Some(Arg::Font(_)) if doc.kind() == 0 => Some(char_fonts(doc)?),
        _ => None,
    };
    let undo_before = undo_entries(doc)?;
    crate::events::clear_result(id);
    doc.post_uno_notify(command.uno, args)?;
    let checked = match command.check {
        Check::Result => match crate::events::take_result(id) {
            Some((_, true)) => Ok(()),
            outcome => refuse(format!("the kernel reported {outcome:?}")),
        },
        Check::History => match history_count(doc, &history)? {
            after if after + 1 == before => Ok(()),
            after => refuse(format!("{history} went {before} to {after}")),
        },
        Check::Undoable => match new_steps(&undo_before, &undo_entries(doc)?) {
            0 => refuse("the undo history did not move".into()),
            _ => Ok(()),
        },
        Check::Listed => {
            let json = doc.command_values(".uno:StyleApply")?;
            let styles = crate::commands::styles(&json, values[1])?;
            if styles.iter().any(|name| name == values[0]) {
                Ok(())
            } else {
                refuse(format!("the style list lacks {}", values[0]))
            }
        }
        Check::Finished => match crate::events::take_result(id) {
            Some(_) => Ok(()),
            None => refuse("the kernel reported no completion".into()),
        },
        Check::Cropping => match crate::events::wait_cropping(id, STATE_WAIT) {
            true => Ok(()),
            false => refuse("the image did not enter crop mode".into()),
        },
        Check::Reports => match &fonts_before {
            // As Word: Western set, Asian set or kept.
            Some(Some([_, asian_before])) => match char_fonts(doc)? {
                Some([western, asian])
                    if western == values[0] && (asian == values[0] || asian == *asian_before) =>
                {
                    Ok(())
                }
                fonts => refuse(format!("the fonts are {fonts:?}, not {}", values[0])),
            },
            // Too long to read: the history must move.
            Some(None) => match new_steps(&undo_before, &undo_entries(doc)?) {
                0 => refuse("the undo history did not move".into()),
                _ => {
                    log::info!("lok-worker: {} past the font lookup limit", command.id);
                    Ok(())
                }
            },
            None => {
                let want = reported_form(&command.args[0], values[0]);
                match crate::events::wait_state(id, command.uno, &want, STATE_WAIT) {
                    Some(state) if state == want => Ok(()),
                    state => refuse(format!("the kernel reports {state:?}, not {want}")),
                }
            }
        },
    };
    if checked.is_err() && command.check != Check::History {
        let steps = new_steps(&undo_before, &undo_entries(doc)?);
        for _ in 0..steps {
            doc.post_uno(".uno:Undo", None)?;
        }
        log::warn!("lok-worker: {} failed, {steps} steps undone", command.id);
    }
    checked
}

/// Calc undo acts on the sheet or the edited cell.
fn sheet_history(id: u64, doc: &Document, command: &Command, args: Option<&str>) -> io::Result<()> {
    let gone = || io::Error::other("the document left the event watch");
    let history = format!("{}Count", command.uno);
    let (count, text) = (
        history_count(doc, &history)?,
        crate::events::a11y_text(id).ok_or_else(gone)?,
    );
    doc.post_uno(command.uno, args)?;
    if history_count(doc, &history)? + 1 == count {
        return Ok(());
    }
    let after = crate::events::wait_a11y_change(id, &text, STATE_WAIT).ok_or_else(gone)?;
    if after == text {
        // An empty history in no cell edit: nothing to do.
        let nothing = if count == 0 { ipc::NOTHING } else { "" };
        return Err(io::Error::other(format!("{nothing}{}: neither {history} nor the cell edit moved", command.id)));
    }
    log::info!("lok-worker: {} in a cell edit", command.id);
    Ok(())
}

/// The kernel's synchronous history depth.
fn history_count(doc: &Document, command: &str) -> io::Result<usize> {
    let count = doc.command_values(command)?;
    count
        .trim()
        .parse()
        .map_err(|_| io::Error::other(format!("a bad {command}: {count}")))
}

/// How the kernel spells a value in its state.
fn reported_form(arg: &Arg, value: &str) -> String {
    match arg {
        Arg::Points(_) => value
            .parse::<f64>()
            .map_or_else(|_| value.to_string(), |points| points.to_string()),
        _ => value.to_string(),
    }
}

/// Writer's Western and Asian fonts, unless past its lookup limit.
fn char_fonts(doc: &Document) -> io::Result<Option<[String; 2]>> {
    crate::commands::char_fonts(&doc.command_values(".uno:CharFonts")?)
}

/// Whether the kernel's font list names `family`.
fn font_listed(doc: &Document, family: &str) -> io::Result<bool> {
    let json = doc.command_values(".uno:CharFontName")?;
    Ok(crate::commands::fonts(&json)?
        .iter()
        .any(|name| name == family))
}

/// A kernel list: fonts or styles (LOK-05).
pub(super) fn values(unpack: &mut Unpack) -> io::Result<Vec<u8>> {
    let (id, command) = (unpack.u64()?, unpack.str()?);
    let base = command.split('?').next().expect("split has a first part");
    if !crate::commands::VALUES.contains(&command.as_str())
        && !crate::commands::QUERIES.contains(&base)
    {
        return Err(invalid(&format!("{command} is not a readable list")));
    }
    let json = with_doc(id, |doc| doc.command_values(&command))??;
    Ok(Pack::new().str(&json).bytes())
}

/// Copies the selection; its flavors come back (LOK-06).
pub(super) fn copy(unpack: &mut Unpack) -> io::Result<Vec<u8>> {
    let id = unpack.u64()?;
    let entries = with_doc(id, |doc| {
        if doc.selection_type() == 0 {
            return Ok(Vec::new());
        }
        doc.post_uno(".uno:Copy", None)?;
        let entries = doc.clipboard(&CLIP_MIMES)?;
        if entries.is_empty() {
            return Err(io::Error::other("the copy produced no flavor"));
        }
        Ok(entries)
    })??
    .into_iter()
    .map(|(mime, mut data)| {
        // LOK's HTML ends in a NUL.
        if mime.starts_with("text/") {
            while data.last() == Some(&0) {
                data.pop();
            }
        }
        (mime, data)
    })
    .collect::<Vec<_>>();
    log::info!("lok-worker: copied {} flavors", entries.len());
    let pack = Pack::new().u64(entries.len() as u64);
    Ok(entries
        .iter()
        .fold(pack, |pack, (mime, data)| pack.str(mime).blob(data))
        .bytes())
}

/// Cuts the selection; a selection left behind is an error.
pub(super) fn cut(unpack: &mut Unpack) -> io::Result<Vec<u8>> {
    let id = unpack.u64()?;
    with_doc(id, |doc| {
        doc.post_uno(".uno:Cut", None)?;
        if doc.selection_type() != 0 {
            return Err(io::Error::other("the selection survived the cut"));
        }
        Ok(())
    })??;
    Ok(Vec::new())
}

/// The undo entries, newest first.
fn undo_entries(doc: &Document) -> io::Result<Vec<Step>> {
    crate::history::steps(&doc.command_values(".uno:Undo")?)
}

/// Entries added since `before`; a full history drops old ones.
fn new_steps(before: &[Step], after: &[Step]) -> usize {
    match before.first() {
        Some(top) => after
            .iter()
            .position(|entry| entry == top)
            .unwrap_or(after.len()),
        None => after.len(),
    }
}

/// Every image a `data:` URI carries must decode.
fn check_html_images(html: &str) -> io::Result<()> {
    use base64::Engine;
    for (at, _) in html.match_indices("data:image/") {
        let rest = &html[at..];
        let Some(comma) = rest.find(";base64,") else {
            return Err(invalid("an inline image is not base64"));
        };
        let body = &rest[comma + 8..];
        let end = body
            .find(|c: char| c == '"' || c == '\'' || c == ')' || c.is_whitespace())
            .unwrap_or(body.len());
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&body[..end])
            .map_err(|error| invalid(&format!("an inline image is bad base64: {error}")))?;
        image::load_from_memory(&bytes)
            .map_err(|error| invalid(&format!("an inline image does not decode: {error}")))?;
    }
    Ok(())
}

/// Refuses payloads the engine would drop half way.
fn check_payload(mime: &str, data: &[u8]) -> io::Result<()> {
    if data.is_empty() {
        return Err(invalid("an empty paste"));
    }
    if mime == "image/png" {
        image::load_from_memory_with_format(data, image::ImageFormat::Png)
            .map_err(|error| invalid(&format!("the png does not decode: {error}")))?;
        return Ok(());
    }
    let text = std::str::from_utf8(data).map_err(|_| invalid("the text is not UTF-8"))?;
    if mime == "text/html" {
        check_html_images(text)?;
    }
    Ok(())
}

/// Pastes one flavor; a failed import is undone and reported.
pub(super) fn paste(unpack: &mut Unpack) -> io::Result<Vec<u8>> {
    let (id, mime, data) = (unpack.u64()?, unpack.str()?, unpack.blob()?);
    if !CLIP_MIMES.contains(&mime.as_str()) {
        return Err(invalid("unsupported paste flavor"));
    }
    check_payload(&mime, &data)?;
    with_doc(id, |doc| {
        let before = undo_entries(doc)?;
        doc.set_clipboard(&mime, &data)?;
        doc.post_uno(".uno:Paste", Some(PASTE_ARGS))?;
        let steps = new_steps(&before, &undo_entries(doc)?);
        // A pasted image ends selected.
        let landed = steps > 0 && (mime != "image/png" || doc.selection_type() == 3);
        if landed {
            return Ok(());
        }
        for _ in 0..steps {
            doc.post_uno(".uno:Undo", None)?;
        }
        log::warn!("lok-worker: paste of {mime} failed, {steps} steps undone");
        Err(io::Error::other(format!("the engine did not paste {mime}")))
    })??;
    log::info!("lok-worker: pasted {mime}, {} bytes", data.len());
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(comment: &str, time: &str) -> (String, String) {
        (comment.into(), time.into())
    }

    #[test]
    fn new_steps_survive_a_full_history() {
        let old = vec![entry("Typing", "1"), entry("Typing", "0")];
        let grown = vec![entry("Paste", "2"), entry("Typing", "1")];
        assert_eq!(new_steps(&old, &grown), 1, "the cap dropped the oldest");
        assert_eq!(new_steps(&old, &old), 0);
        assert_eq!(new_steps(&[], &grown), 2);
    }

    #[test]
    fn inline_images_must_decode() {
        let bad = r#"<img src="data:image/png;base64,dGhpcyBpcyBub3QgYSBwbmc=">"#;
        assert!(check_payload("text/html", bad.as_bytes()).is_err());
        assert!(check_payload("text/html", b"<b>plain</b>").is_ok());
        assert!(check_payload("text/html", br#"<img src="data:image/png,raw">"#).is_err());
    }
}
