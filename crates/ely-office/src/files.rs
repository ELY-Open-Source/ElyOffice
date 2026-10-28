//! Formats and the kernel half of a save (FILE-04).

use std::{
    borrow::Cow,
    io,
    path::{Path, PathBuf},
};

use gpui::SharedString;
use quick_xml::{
    events::Event,
    name::{Namespace, ResolveResult},
    reader::NsReader,
};

use crate::{Engine, Kind};

mod zip;

/// The lowercase extension of `path`.
fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_lowercase()
}

/// The format a save of `path` writes: its own (FILE-04).
pub fn kept_format(kind: &Kind, path: &Path) -> io::Result<&'static str> {
    let ext = extension(path);
    kind.kept
        .iter()
        .find(|kept| **kept == ext)
        .copied()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, format!("cannot save .{ext}")))
}

/// The format a Save As to `path` writes, or None.
pub fn save_as_format(kind: &Kind, path: &Path) -> Option<&'static str> {
    let ext = extension(path);
    kind.save_as.iter().find(|format| **format == ext).copied()
}

/// The kernel writes `format` into `work`; the bytes come back.
pub fn kernel_bytes<G: Engine>(
    worker: &mut G::Worker,
    doc: &G::Doc,
    work: &Path,
    format: &str,
) -> io::Result<Vec<u8>> {
    let copy = work.join(format!("save.{format}"));
    G::write(worker, doc, &copy, format)?;
    let bytes = std::fs::read(&copy)?;
    log::info!("office: the kernel wrote {} bytes of {format}", bytes.len());
    Ok(bytes)
}

/// Whether `path` carries macros Ely will not round-trip (LOK-11).
pub fn macro_risk(kind: &Kind, path: &Path) -> io::Result<bool> {
    let Some(macros) = kind.macros else {
        return Ok(false);
    };
    if macros.contains(&extension(path).as_str()) {
        return Ok(true);
    }
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(b"PK\x03\x04") {
        let names = zip::names(&bytes)?;
        return Ok(names
            .iter()
            .any(|name| name.ends_with("vbaProject.bin") || name.starts_with("Basic/")));
    }
    if bytes.starts_with(OLE) {
        return Ok(ole_has(&bytes, "_VBA_PROJECT"));
    }
    flat_odf_has_basic(&bytes)
}

/// The first bytes of an OLE compound file.
const OLE: &[u8] = &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

/// The office namespace that carries Basic libraries.
const OOO: &[u8] = b"http://openoffice.org/2004/office";

/// Whether flat ODF embeds a Basic library, namespaces in scope.
fn flat_odf_has_basic(bytes: &[u8]) -> io::Result<bool> {
    let text = utf8(bytes)?;
    if !text.trim_ascii_start().starts_with(b"<") {
        return Ok(false);
    }
    holds(&text, OOO, b"library-embedded")
}

/// XML bytes in an ASCII-compatible form; UTF-16 decoded.
fn utf8(bytes: &[u8]) -> io::Result<Cow<'_, [u8]>> {
    let utf16 = |encoding: &'static encoding_rs::Encoding, rest: &[u8]| {
        let text = encoding.decode_without_bom_handling_and_without_replacement(rest);
        let text = text.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("bad {}", encoding.name())))?;
        Ok(Cow::Owned(text.into_owned().into_bytes()))
    };
    match bytes {
        [0xFF, 0xFE, 0, 0, ..] | [0, 0, 0xFE, 0xFF, ..] | [0x3C, 0, 0, 0, ..] | [0, 0, 0, 0x3C, ..] => {
            Err(io::Error::new(io::ErrorKind::InvalidData, "UTF-32 XML is not read"))
        }
        [0xEF, 0xBB, 0xBF, rest @ ..] => Ok(Cow::Borrowed(rest)),
        [0xFF, 0xFE, rest @ ..] => utf16(encoding_rs::UTF_16LE, rest),
        [0xFE, 0xFF, rest @ ..] => utf16(encoding_rs::UTF_16BE, rest),
        [0x3C, 0, ..] => utf16(encoding_rs::UTF_16LE, bytes),
        [0, 0x3C, ..] => utf16(encoding_rs::UTF_16BE, bytes),
        _ => Ok(Cow::Borrowed(bytes)),
    }
}

/// Whether XML `text` has element `name` in namespace `ns`.
fn holds(text: &[u8], ns: &[u8], name: &[u8]) -> io::Result<bool> {
    let text = utf8(text)?;
    let mut reader = NsReader::from_reader(text.as_ref());
    let mut buf = Vec::new();
    loop {
        let (bound, event) = reader
            .read_resolved_event_into(&mut buf)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        match (bound, event) {
            (ResolveResult::Bound(Namespace(found)), Event::Start(tag) | Event::Empty(tag))
                if found == ns && tag.local_name().as_ref() == name =>
            {
                return Ok(true);
            }
            (_, Event::Eof) => return Ok(false),
            _ => {}
        }
        buf.clear();
    }
}

/// PowerPoint 2010's namespace, which holds sections.
const P14: &[u8] = b"http://schemas.microsoft.com/office/powerpoint/2010/main";

/// Whether a PPTX has slide sections, which the kernel drops.
pub fn sections(path: &Path) -> io::Result<bool> {
    let bytes = std::fs::read(path)?;
    if !bytes.starts_with(b"PK\x03\x04") {
        return Ok(false);
    }
    match zip::read(&bytes, "ppt/presentation.xml")? {
        Some(xml) => holds(&xml, P14, b"sectionLst"),
        None => Ok(false),
    }
}

/// What ElyShow's file risks: sections, or a seal hiding them.
pub fn section_risk(kind: &Kind, path: &Path) -> io::Result<Option<crate::Risk>> {
    if kind.app != ely_platform::AppId::Show {
        return Ok(None);
    }
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(OLE) && ole_has(&bytes, "EncryptedPackage") {
        return Ok(Some(crate::Risk::Sealed));
    }
    Ok(sections(path)?.then_some(crate::Risk::Sections))
}

/// Whether an OLE file has a stream entry `name`.
fn ole_has(bytes: &[u8], name: &str) -> bool {
    let name: Vec<u8> = name
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    // 128-byte entries: name, byte length, type 2.
    (512..bytes.len().saturating_sub(128))
        .step_by(128)
        .any(|at| {
            let entry = &bytes[at..at + 128];
            entry.starts_with(&name)
                && u16::from_le_bytes([entry[64], entry[65]]) as usize == name.len() + 2
                && entry[66] == 2
        })
}

/// A path's file name for tab titles.
pub fn file_name(path: &Path) -> SharedString {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
        .into()
}

/// A fresh scratch folder for one document's kernel copies.
pub fn work_dir(root: &Path, id: u64) -> io::Result<PathBuf> {
    let dir = root.join(format!("doc-{id}"));
    if dir.try_exists()? {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KIND: Kind = Kind {
        app: ely_platform::AppId::Doc,
        kept: &["docx", "odt", "doc"],
        save_as: &["docx", "odt"],
        macros: Some(&["docm", "dotm"]),
    };

    #[test]
    fn saves_keep_the_format_and_save_as_converts() {
        let kept = |path: &str| kept_format(&KIND, Path::new(path));
        assert_eq!(kept("/a/Contract.DOC").expect("doc"), "doc");
        assert_eq!(kept("/a/b.odt").expect("odt"), "odt");
        assert!(kept("/a/b.pages").is_err(), "unknown formats fail");
        assert_eq!(save_as_format(&KIND, Path::new("/a/b.docx")), Some("docx"));
        assert_eq!(save_as_format(&KIND, Path::new("/a/b.pdf")), None);
    }

    #[test]
    fn macros_are_found_by_structure_never_by_text() {
        let dir = std::env::temp_dir().join(format!("ely-office-risk-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a dir");
        let write = |name: &str, body: &[u8]| {
            let path = dir.join(name);
            std::fs::write(&path, body).expect("writes");
            path
        };
        assert!(macro_risk(&KIND, &write("a.docm", b"PK")).expect("docm"));
        let notes = write(
            "notes.txt",
            b"The archive path is Basic/Standard/Module1.xml and vbaProject.bin",
        );
        assert!(
            !macro_risk(&KIND, &notes).expect("text"),
            "words in text are not macros"
        );
        let example = write(
            "xml-example.txt",
            b"Example: <ooo:library-embedded ooo:name=\"Standard\">",
        );
        assert!(
            !macro_risk(&KIND, &example).expect("text with a tag"),
            "unbound tags are text"
        );
        let root = |prefix: &str| {
            format!(
                "<office:document xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:{prefix}=\"http://openoffice.org/2004/office\">"
            )
        };
        let basic = format!(
            "{}<ooo:libraries><ooo:library-embedded ooo:name=\"Standard\"/>",
            root("ooo")
        );
        assert!(macro_risk(&KIND, &write("f.fodt", basic.as_bytes())).expect("flat basic"));
        let alias = format!(
            "{}<basic:libraries><basic:library-embedded basic:name=\"Standard\"/>",
            root("basic")
        );
        assert!(
            macro_risk(&KIND, &write("g.fodt", alias.as_bytes())).expect("aliased"),
            "any bound prefix counts"
        );
        let nested = format!(
            "{}<office:scripts><office:script xmlns:x=\"http://openoffice.org/2004/office\"><x:libraries><x:library-embedded x:name=\"Standard\"/>",
            root("ooo")
        );
        assert!(
            macro_risk(&KIND, &write("n.fods", nested.as_bytes())).expect("nested"),
            "a child may declare the namespace"
        );
        let shadowed = format!(
            "{}<ooo:p xmlns:ooo=\"urn:other\"><ooo:library-embedded/></ooo:p>",
            root("ooo")
        );
        assert!(
            !macro_risk(&KIND, &write("s.fodt", shadowed.as_bytes())).expect("shadowed"),
            "a prefix rebound elsewhere is not Basic"
        );
        assert!(
            macro_risk(&KIND, &write("bad.fodt", b"<office:document><a></b>")).is_err(),
            "broken XML is an error"
        );
        std::fs::remove_dir_all(&dir).expect("cleans");
    }

    /// A zip of one stored entry, CRC unchecked.
    fn stored(name: &str, data: &[u8]) -> Vec<u8> {
        let (size, len) = ((data.len() as u32).to_le_bytes(), (name.len() as u16).to_le_bytes());
        let mut zip = b"PK\x03\x04\x14\0\0\0\0\0\0\0\0\0\0\0\0\0".to_vec();
        zip.extend(size.into_iter().chain(size).chain(len).chain([0, 0]));
        zip.extend(name.as_bytes().iter().chain(data));
        let dir = zip.len() as u32;
        zip.extend(b"PK\x01\x02\x14\0\x14\0\0\0\0\0\0\0\0\0\0\0\0\0");
        zip.extend(size.into_iter().chain(size).chain(len).chain([0; 16]));
        zip.extend(name.as_bytes());
        let dir_size = zip.len() as u32 - dir;
        zip.extend(b"PK\x05\x06\0\0\0\0\x01\0\x01\0");
        zip.extend(dir_size.to_le_bytes().into_iter().chain(dir.to_le_bytes()).chain([0, 0]));
        zip
    }

    #[test]
    fn sections_are_found_in_the_presentation_part() {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
        assert!(sections(&fixtures.join("show/sections.pptx")).expect("deflated"), "two sections");
        assert!(!sections(&fixtures.join("office/t.pptx")).expect("pptx"), "no sections");
        assert!(!sections(&fixtures.join("show/trouble.fodp")).expect("flat odp"));
        assert!(!sections(&fixtures.join("office/t.docx")).expect("docx"), "no presentation part");
        let dir = tempfile::tempdir().expect("a temp dir");
        let write = |name: &str, xml: &str| {
            let path = dir.path().join(name);
            std::fs::write(&path, stored("ppt/presentation.xml", xml.as_bytes())).expect("writes");
            path
        };
        let deck = |inner: &str| format!(r#"<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main">{inner}</p:presentation>"#);
        assert!(sections(&write("s.pptx", &deck("<p14:sectionLst/>"))).expect("stored"), "a stored part");
        assert!(!sections(&write("p.pptx", &deck("<p:sectionLst/>"))).expect("other ns"), "only p14 counts");
        assert!(sections(&write("b.pptx", &deck("<a></b><p14:sectionLst/>"))).is_err(), "broken XML is an error");
        let raw = |name: &str, xml: Vec<u8>| {
            let path = dir.path().join(name);
            std::fs::write(&path, stored("ppt/presentation.xml", &xml)).expect("writes");
            path
        };
        let text = deck("<p14:sectionLst/>");
        let le: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let be: Vec<u8> = text.encode_utf16().flat_map(u16::to_be_bytes).collect();
        assert!(sections(&raw("le.pptx", [&[0xFF, 0xFE][..], &le].concat())).expect("UTF-16LE"));
        assert!(sections(&raw("le-bare.pptx", le.clone())).expect("UTF-16LE, no BOM"));
        assert!(sections(&raw("be.pptx", [&[0xFE, 0xFF][..], &be].concat())).expect("UTF-16BE"));
        let utf32: Vec<u8> = text.chars().flat_map(|c| (c as u32).to_le_bytes()).collect();
        assert!(sections(&raw("32.pptx", utf32)).is_err(), "UTF-32 is an error, not absence");
        assert!(sections(&raw("bad.pptx", vec![0xFF, 0xFE, 0x3C, 0, 0x00, 0xD8])).is_err(), "a lone surrogate");
    }

    #[test]
    fn only_show_asks_and_a_seal_hides_the_sections() {
        let show = Kind { app: ely_platform::AppId::Show, kept: &["pptx"], save_as: &["pptx"], macros: None };
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/show");
        let risk = |kind: &Kind, name: &str| section_risk(kind, &fixtures.join(name)).expect(name);
        assert_eq!(risk(&show, "sections.pptx"), Some(crate::Risk::Sections));
        assert_eq!(risk(&show, "sections-utf16.pptx"), Some(crate::Risk::Sections));
        assert_eq!(risk(&show, "sections-locked.pptx"), Some(crate::Risk::Sealed));
        assert_eq!(risk(&show, "trouble.fodp"), None);
        assert_eq!(risk(&KIND, "sections.pptx"), None, "Doc and Table never ask");
    }
}
