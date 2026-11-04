//! Word's linked objects the kernel cannot keep linked (0088).

use std::{io, path::Path};

use quick_xml::{
    events::{BytesStart, Event},
    name::{Namespace, ResolveResult},
    reader::NsReader,
};

use crate::{Kind, Risk};

/// Office's VML namespace, which holds OLEObject.
const OFFICE: &[u8] = b"urn:schemas-microsoft-com:office:office";

/// The kinds 0088 links; its table in OLEHandler.cxx.
const LINKED: &[&[u8]] = &[
    b"Excel.Sheet.12",
    b"Excel.Sheet.8",
    b"Word.Document.12",
    b"Word.Document.8",
    b"PowerPoint.Show.12",
    b"PowerPoint.Show.8",
    b"Excel.SheetMacroEnabled.12",
    b"Excel.SheetBinaryMacroEnabled.12",
    b"Word.DocumentMacroEnabled.12",
    b"PowerPoint.ShowMacroEnabled.12",
];

/// What ElyDoc's DOCX risks: links the kernel keeps as pictures.
pub fn link_risk(kind: &Kind, path: &Path) -> io::Result<Option<Risk>> {
    if kind.app != ely_platform::AppId::Doc {
        return Ok(None);
    }
    let bytes = std::fs::read(path)?;
    if !bytes.starts_with(b"PK\x03\x04") {
        return Ok(None);
    }
    for name in super::zip::names(&bytes)? {
        if !(name.starts_with("word/") && name.ends_with(".xml")) {
            continue;
        }
        if let Some(xml) = super::zip::read(&bytes, &name)?
            && unkept(&xml)?
        {
            log::warn!("doc: {name} links an object the kernel keeps as a picture");
            return Ok(Some(Risk::Links));
        }
    }
    Ok(None)
}

/// Whether `xml` links a kind 0088 keeps as a picture.
fn unkept(xml: &[u8]) -> io::Result<bool> {
    let text = super::utf8(xml)?;
    let mut reader = NsReader::from_reader(text.as_ref());
    let mut buf = Vec::new();
    loop {
        let (bound, event) = reader
            .read_resolved_event_into(&mut buf)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        match (bound, event) {
            (ResolveResult::Bound(Namespace(ns)), Event::Start(tag) | Event::Empty(tag))
                if ns == OFFICE && tag.local_name().as_ref() == b"OLEObject" =>
            {
                if value(&tag, "Type")?.as_deref() == Some(b"Link".as_slice())
                    && !value(&tag, "ProgID")?.is_some_and(|prog| LINKED.contains(&prog.as_slice()))
                {
                    return Ok(true);
                }
            }
            (_, Event::Eof) => return Ok(false),
            _ => {}
        }
        buf.clear();
    }
}

fn value(tag: &BytesStart, key: &str) -> io::Result<Option<Vec<u8>>> {
    let found = tag.try_get_attribute(key).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(found.map(|attr| attr.value.into_owned()))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::link_risk;
    use crate::{Kind, Risk};

    #[test]
    fn a_link_of_an_unknown_kind_is_a_risk_in_doc_only() {
        let doc = Kind { app: ely_platform::AppId::Doc, kept: &["docx"], save_as: &["docx"], macros: None };
        let show = Kind { app: ely_platform::AppId::Show, kept: &["pptx"], save_as: &["pptx"], macros: None };
        let office = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office");
        let risk = |kind: &Kind, name: &str| link_risk(kind, &office.join(name)).expect(name);
        assert_eq!(risk(&doc, "ole-link-other.docx"), Some(Risk::Links));
        assert_eq!(risk(&doc, "ole-link.docx"), None, "a linked worksheet stays a link");
        assert_eq!(risk(&doc, "includetext.docx"), None);
        assert_eq!(risk(&show, "ole-link-other.docx"), None, "only ElyDoc asks");
    }
}
