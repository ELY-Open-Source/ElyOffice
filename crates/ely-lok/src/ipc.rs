pub use ely_ipc::{MAX_FRAME, Pack, UNASKED, Unpack, check_reply, read_frame, write_frame};

/// A local path as a `file:` URL, UTF-8 escaped.
pub fn file_url_of(path: &std::path::Path) -> String {
    let text = path.to_string_lossy().into_owned();
    #[cfg(windows)]
    let text = if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        format!("file://{}", rest.replace('\\', "/"))
    } else if let Some(rest) = text.strip_prefix(r"\\?\") {
        format!("file:///{}", rest.replace('\\', "/"))
    } else {
        format!("file:///{}", text.replace('\\', "/"))
    };
    #[cfg(not(windows))]
    let text = format!("file://{text}");
    let mut url = String::new();
    for byte in text.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                url.push(*byte as char)
            }
            other => url.push_str(&format!("%{other:02X}")),
        }
    }
    url
}

/// The path a `file:` URL names; other URLs fail.
pub fn path_of_file_url(url: &str) -> std::io::Result<std::path::PathBuf> {
    let bad = || std::io::Error::other(format!("{url} is no file URL"));
    let rest = url.strip_prefix("file://").ok_or_else(bad)?;
    let mut bytes = Vec::with_capacity(rest.len());
    let mut iter = rest.bytes();
    while let Some(byte) = iter.next() {
        if byte != b'%' {
            bytes.push(byte);
            continue;
        }
        let hex = [iter.next().ok_or_else(bad)?, iter.next().ok_or_else(bad)?];
        let hex = std::str::from_utf8(&hex).map_err(|_| bad())?;
        bytes.push(u8::from_str_radix(hex, 16).map_err(|_| bad())?);
    }
    let text = String::from_utf8(bytes).map_err(|_| bad())?;
    #[cfg(windows)]
    let text = text.strip_prefix('/').unwrap_or(&text).replace('/', "\\");
    Ok(std::path::PathBuf::from(text))
}

/// The load error a missing or wrong password gives.
pub const LOCKED: &str = "the document needs its password";
/// The load error a broken package gives.
pub const DAMAGED: &str = "the file is damaged";
/// The error of a snapshot that gave way to input.
pub const STOPPED: &str = "the snapshot gave way to input";

/// Request tags from the client.
pub mod tag {
    pub const BUILD_ID: u8 = 1;
    pub const LOAD: u8 = 2;
    pub const DOC_SIZE: u8 = 3;
    pub const SAVE_AS: u8 = 4;
    pub const CLOSE_DOC: u8 = 5;
    pub const SHUTDOWN: u8 = 6;
    pub const VERIFY: u8 = 7;
    pub const PAINT_TILE: u8 = 12;
    /// Unasked: one document's new events.
    pub const EVENTS: u8 = 13;
    pub const POST_UNO: u8 = 14;
    pub const POST_MOUSE: u8 = 8;
    pub const RUN_MACRO: u8 = 10;
    pub const POST_KEY: u8 = 11;
    pub const POST_TEXT: u8 = 15;
    pub const COPY: u8 = 16;
    pub const PASTE: u8 = 17;
    pub const CUT: u8 = 18;
    pub const VALUES: u8 = 19;
    pub const PAGES: u8 = 20;
    pub const GRAPHIC: u8 = 21;
    pub const EXPORT_PDF: u8 = 22;
    pub const SHEETS: u8 = 23;
    pub const SHOW_SHEET: u8 = 24;
    pub const VISIBLE_AREA: u8 = 25;
    pub const SELECTION_TEXT: u8 = 26;
    pub const SET_AUTHOR: u8 = 27;
    pub const COMPLETE_FUNCTION: u8 = 28;
    pub const IMPORT: u8 = 29;
    pub const EXPORT_TEXT: u8 = 30;
    pub const PAINT_PART: u8 = 31;
    pub const SLIDE_LAYERS: u8 = 32;
    /// Refuses unless the document is still at a revision.
    pub const EXPECT: u8 = 33;
    pub const REVISION: u8 = 34;
    /// One layer's pixels from the last `SLIDE_LAYERS`.
    pub const SLIDE_LAYER: u8 = 35;
    /// Turns parts; an arrow with something picked goes in.
    pub const TURN: u8 = 36;
    /// Unanswered: a request waits (1) or holds the lock (0).
    pub const WAIT: u8 = 37;
    pub const OK: u8 = 100;
    pub const ERR: u8 = 101;
}

/// How a refusal for a changed document begins.
pub const STALE: &str = "stale: ";
/// How a refusal for an empty undo or redo begins.
pub const NOTHING: &str = "nothing: ";

#[cfg(test)]
mod tests {
    #[test]
    fn a_file_url_turns_back_into_its_path() {
        let path = std::env::temp_dir().join("幻灯片 one%.mp4");
        let url = super::file_url_of(&path);
        assert_eq!(super::path_of_file_url(&url).expect("decodes"), path);
        assert!(super::path_of_file_url("vnd.sun.star.Package:Media/a.mp4").is_err());
        assert!(super::path_of_file_url("file:///bad%zz").is_err());
    }
}
